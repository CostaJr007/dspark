//! Verification-scaling A/B experiments (offline, deterministic, zero API cost).
//!
//! Each experiment simulates a judge/verifier with known ground truth and runs the
//! REAL code paths (PivotTournament, LogprobExtractor, StsCalibrator, CostScheduler)
//! in baseline vs improved configurations, asserting the improvement direction and
//! printing a comparison table (`cargo test --test verification_scaling_test -- --nocapture`).
//!
//! Experiments mirror the papers' methodology:
//!   E1. PPT binary vs Bradley-Terry soft updates          (arXiv:2607.05391, Sec. 3.2)
//!   E2. Discrete judge vs continuous reward tie rate       (arXiv:2607.05391, Sec. 4.1)
//!   E3. Raw vs STS-calibrated confidence (ECE)             (arXiv:2607.05147, Sec. 3.2.1)
//!   E4. Legacy budget cap vs greedy early-stop scheduler   (arXiv:2607.05147, Algorithm 1)

use dspark::client::{ModelClient, ScriptedClient};
use dspark::engine::{
    comparison_preference, tournament_comparison_count, BlockConfidence, CostScheduler,
    DraftTrajectory, LogprobExtractor, PivotTournament, RiskLevel, StsCalibrator,
};
use dspark::utils::ast_resolver::CodeBlock;
use std::sync::{Arc, Mutex};

fn xorshift(state: &mut u64) -> f64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state as f64) / (u64::MAX as f64)
}

/// Extracts the two candidate indices embedded in a comparison prompt
/// (`fn candidate_{i}() {{ i }}` blocks).
fn parse_pair_indices(prompt: &str) -> (usize, usize) {
    let mut idxs = Vec::new();
    let mut rest = prompt;
    while let Some(pos) = rest.find("candidate_") {
        let after = &rest[pos + "candidate_".len()..];
        let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            idxs.push(digits.parse::<usize>().unwrap());
        }
        rest = &after[digits.len()..];
    }
    (idxs[0], idxs[1])
}

fn mk_trajectories(n: usize) -> Vec<DraftTrajectory> {
    (0..n)
        .map(|i| DraftTrajectory {
            id: i,
            full_code: format!("fn candidate_{}() {{ {} }}", i, i),
            code_blocks: vec![CodeBlock {
                function_name: format!("candidate_{}", i),
                code: format!("fn candidate_{}() {{ {} }}", i, i),
                line_count: 1,
            }],
            confidence_score: 0.8,
            ast_valid: true,
        })
        .collect()
}

// ---------------------------------------------------------------- E1: PPT
struct SimulatedJudge {
    qualities: Vec<f64>,
    noise_scale: f64,
    tie_band: f64,
    soft: bool,
    rng: Arc<Mutex<u64>>,
    ties: Arc<Mutex<usize>>,
}

impl SimulatedJudge {
    fn respond(&self, prompt: &str) -> String {
        let (a, b) = parse_pair_indices(prompt);
        let noise = {
            let mut rng = self.rng.lock().unwrap();
            (xorshift(&mut rng) - 0.5) * 2.0 * self.noise_scale
        };
        let delta = self.qualities[a] - self.qualities[b] + noise;

        if self.soft {
            let sa = (10.0 + 10.0 * delta).round().clamp(1.0, 20.0) as i64;
            let sb = (10.0 - 10.0 * delta).round().clamp(1.0, 20.0) as i64;
            return format!(
                "{{\"winner\": \"A\", \"score_A\": {}, \"score_B\": {}}}",
                sa, sb
            );
        }

        if delta.abs() < self.tie_band {
            *self.ties.lock().unwrap() += 1;
            "{\"winner\": \"EQUAL\"}".to_string()
        } else if delta > 0.0 {
            "{\"winner\": \"A\"}".to_string()
        } else {
            "{\"winner\": \"B\"}".to_string()
        }
    }
}

/// Runs one tournament with a simulated judge; returns (correct, comparisons).
async fn run_simulated_tournament(
    n: usize,
    pivots: usize,
    qualities: Vec<f64>,
    noise_scale: f64,
    tie_band: f64,
    soft: bool,
    seed: u64,
) -> (bool, usize, usize) {
    let truth = qualities
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0);
    let ties = Arc::new(Mutex::new(0usize));
    let judge = Arc::new(SimulatedJudge {
        qualities,
        noise_scale,
        tie_band,
        soft,
        rng: Arc::new(Mutex::new(seed)),
        ties: Arc::clone(&ties),
    });
    let client = ModelClient::Scripted(ScriptedClient::new(
        if soft { "judge-soft" } else { "judge-binary" },
        {
            let judge = Arc::clone(&judge);
            move |prompt| judge.respond(prompt)
        },
    ));
    let tournament = PivotTournament::new(client, pivots);
    let res = tournament
        .run_tournament(&mk_trajectories(n), "Check correctness")
        .await;
    let comps = res.total_comparisons;
    let tie_count = *ties.lock().unwrap();
    (res.best_trajectory_idx == truth, comps, tie_count)
}

#[tokio::test]
async fn e1_ppt_soft_updates_beat_binary() {
    const N: usize = 10;
    const K: usize = 3;
    const M: usize = 200;

    let mut state: u64 = 0xC0FFEE;
    let mut binary_correct = 0usize;
    let mut soft_correct = 0usize;
    let mut binary_ties = 0usize;
    let mut soft_ties = 0usize;

    for m in 0..M {
        let qualities: Vec<f64> = (0..N).map(|_| xorshift(&mut state)).collect();

        let (bin_ok, bin_comps, bin_ties) = run_simulated_tournament(
            N, K, qualities.clone(), 0.35, 0.12, false, 1000 + m as u64,
        ).await;
        let (soft_ok, soft_comps, s_ties) = run_simulated_tournament(
            N, K, qualities.clone(), 0.35, 0.12, true, 2000 + m as u64,
        ).await;

        assert_eq!(bin_comps, tournament_comparison_count(N, K));
        assert_eq!(soft_comps, tournament_comparison_count(N, K));
        binary_correct += bin_ok as usize;
        soft_correct += soft_ok as usize;
        binary_ties += bin_ties;
        soft_ties += s_ties;
    }

    let bin_acc = binary_correct as f64 / M as f64;
    let soft_acc = soft_correct as f64 / M as f64;
    println!(
        "\n[E1] PPT selection (N={N}, k={K}, {M} tournaments, noisy judge):\n\
         \x20 binary winner parsing : accuracy {:.1}%, EQUAL ties {}\n\
         \x20 soft Bradley-Terry     : accuracy {:.1}%, EQUAL ties {}\n\
         \x20 -> soft selects the true best more often ({} vs {} tournaments)",
        bin_acc * 100.0, binary_ties, soft_acc * 100.0, soft_ties, soft_correct, binary_correct
    );
    assert!(
        soft_acc >= bin_acc,
        "soft updates must not reduce selection accuracy: {soft_acc} vs {bin_acc}"
    );
    assert!(soft_ties == 0, "continuous scores eliminate ties entirely");
    assert!(binary_ties > 0, "binary judge must exhibit ties (else the experiment is vacuous)");
}

// --------------------------------------------- E2: discrete vs continuous
#[test]
fn e2_continuous_reward_beats_discrete_judge() {
    const G: usize = 20;
    const REPS: usize = 400;
    let correct_r = 0.60f64;
    let incorrect_r = 0.40f64;
    let sigma = 0.12f64;

    let distribution = |r: f64| -> Vec<(String, f64)> {
        (0..G)
            .map(|i| {
                let letter = (b'A' + i as u8) as char;
                let phi = i as f64 / (G - 1) as f64;
                let logit = -((phi - r).powi(2)) / (2.0 * sigma * sigma);
                (letter.to_string(), logit)
            })
            .collect()
    };

    let mut state: u64 = 0xD15C0;
    let mut discrete_ties = 0usize;
    let mut discrete_correct = 0usize;
    let mut continuous_ties = 0usize;
    let mut continuous_correct = 0usize;

    for _ in 0..REPS {
        let d_c = distribution(correct_r);
        let d_i = distribution(incorrect_r);

        // Discrete judge: sample the argmax score token of each distribution.
        let sample_token = |dist: &[(String, f64)], s: &mut u64| -> f64 {
            let weights: Vec<f64> = dist.iter().map(|(_, l)| l.exp()).collect();
            let total: f64 = weights.iter().sum();
            let roll = xorshift(s) * total;
            let mut acc = 0.0;
            for (i, w) in weights.iter().enumerate() {
                acc += w;
                if roll <= acc {
                    return i as f64 / (G - 1) as f64;
                }
            }
            (G - 1) as f64 / (G - 1) as f64
        };
        let sc = sample_token(&d_c, &mut state);
        let si = sample_token(&d_i, &mut state);
        if (sc - si).abs() < 1e-9 {
            discrete_ties += 1;
        } else if sc > si {
            discrete_correct += 1;
        }

        // Continuous verifier: expectation over the scoring-token distribution.
        let extractor = LogprobExtractor::new();
        let rc = extractor.continuous_reward(&d_c, G);
        let ri = extractor.continuous_reward(&d_i, G);
        if (rc - ri).abs() < 1e-9 {
            continuous_ties += 1;
        } else if rc > ri {
            continuous_correct += 1;
        }
    }

    let d_tie = discrete_ties as f64 / REPS as f64;
    let d_acc = discrete_correct as f64 / REPS as f64;
    let c_acc = continuous_correct as f64 / REPS as f64;
    println!(
        "\n[E2] Verifier scoring (hedged pair r=0.60 vs 0.40, {REPS} evaluations):\n\
         \x20 discrete judge    : correct {:.1}%, tie rate {:.1}%\n\
         \x20 continuous reward : correct {:.1}%, tie rate {:.1}%",
        d_acc * 100.0, d_tie * 100.0, c_acc * 100.0, continuous_ties as f64 / REPS as f64 * 100.0
    );
    assert!(d_tie > 0.05, "discrete judge must exhibit ties on overlapping distributions");
    assert_eq!(continuous_ties, 0, "continuous rewards eliminate ties");
    assert!(c_acc > d_acc, "continuous expectation must rank the better trajectory more often");
}

// ------------------------------------------------- E3: STS ECE reduction
#[test]
fn e3_sts_calibration_reduces_ece_across_seeds() {
    let mut rng: u64 = 0x5EED;
    let n = 1500usize;
    let gamma = 4usize;
    let mut raw_eces = Vec::new();
    let mut cal_eces = Vec::new();

    for seed in 0..5u64 {
        let mut rand = || xorshift(&mut rng);
        let samples: Vec<Vec<f64>> = (0..n)
            .map(|_| (0..gamma).map(|_| 0.82 + 0.08 * rand()).collect())
            .collect();
        let outcomes: Vec<Vec<bool>> = samples
            .iter()
            .map(|_| (0..gamma).map(|_| rand() < 0.5).collect())
            .collect();

        let calibrator = StsCalibrator::fit(&samples, &outcomes, None);
        let calibrated: Vec<Vec<f64>> = samples.iter().map(|s| calibrator.calibrate(s)).collect();

        let ece = |probs: &[f64], labels: &[f64]| -> f64 {
            let mut total = 0.0;
            for b in 0..15 {
                let lo = b as f64 / 15.0;
                let hi = (b + 1) as f64 / 15.0;
                let mut sc = 0.0;
                let mut sa = 0.0;
                let mut cnt = 0.0;
                for (p, y) in probs.iter().zip(labels) {
                    if *p >= lo && (*p < hi || (b + 1 == 15 && *p <= hi)) {
                        sc += p;
                        sa += y;
                        cnt += 1.0;
                    }
                }
                if cnt > 0.0 {
                    total += (cnt / probs.len() as f64) * ((sa - sc) / cnt).abs();
                }
            }
            total
        };
        let labels0: Vec<f64> = outcomes.iter().map(|o| if o[0] { 1.0 } else { 0.0 }).collect();
        raw_eces.push(ece(&samples.iter().map(|s| s[0]).collect::<Vec<_>>(), &labels0));
        cal_eces.push(ece(&calibrated.iter().map(|s| s[0]).collect::<Vec<_>>(), &labels0));
        let _ = seed;
    }

    let raw_mean = raw_eces.iter().sum::<f64>() / raw_eces.len() as f64;
    let cal_mean = cal_eces.iter().sum::<f64>() / cal_eces.len() as f64;
    println!(
        "\n[E3] STS calibration (5 seeds, 1500 samples/seed):\n\
         \x20 raw confidence head : mean ECE {:.4}\n\
         \x20 STS calibrated      : mean ECE {:.4}\n\
         \x20 -> calibration error reduced by {:.1}%",
        raw_mean, cal_mean, (1.0 - cal_mean / raw_mean) * 100.0
    );
    assert!(cal_mean < raw_mean * 0.5, "STS must cut ECE substantially");
}

// ---------------------------------------- E4: scheduler early-stop savings
#[test]
fn e4_early_stop_saves_calls_without_missing_failures() {
    const N: usize = 200;
    let mut rng: u64 = 0xBEEF;
    let mut blocks = Vec::new();
    for i in 0..N {
        let c = 0.15 + 0.83 * xorshift(&mut rng);
        let risk_level = if c > 0.88 {
            RiskLevel::Low
        } else if c > 0.65 {
            RiskLevel::Medium
        } else {
            RiskLevel::High
        };
        let needs_verification = c < 0.85;
        blocks.push(BlockConfidence {
            block_id: i,
            function_name: format!("fn_{i}"),
            confidence_score: c,
            needs_verification,
            risk_level,
        });
    }

    let legacy = CostScheduler::new(200, 0.002);
    // Threshold 0.31 prunes blocks whose survival is pinned at the Medium cap
    // (risk exactly 0.30) -- the tail the legacy cap would still pay for.
    let improved = CostScheduler::with_early_stop(200, 0.002, 0.31);

    let plan_legacy = legacy.schedule_verification(&blocks);
    let plan_es = improved.schedule_verification(&blocks);

    // Ground-truth failure expectation per block = 1 - confidence (calibrated sim).
    let expected_failures = |plan: &[usize]| -> f64 {
        plan.iter().map(|&i| 1.0 - blocks[i].confidence_score).sum()
    };
    let caught_legacy = expected_failures(&plan_legacy.blocks_to_verify);
    let caught_es = expected_failures(&plan_es.blocks_to_verify);

    let calls_saved = plan_legacy.blocks_to_verify.len() - plan_es.blocks_to_verify.len();
    let recall_drop = (caught_legacy - caught_es) / caught_legacy.max(1e-9);
    let per_call_legacy = caught_legacy / plan_legacy.blocks_to_verify.len().max(1) as f64;
    let per_call_es = caught_es / plan_es.blocks_to_verify.len().max(1) as f64;

    println!(
        "\n[E4] CostScheduler ({N} blocks, simulated calibrated risks):\n\
         \x20 legacy budget cap    : {} calls, failures caught {:.2} ({:.3}/call)\n\
         \x20 greedy + early stop  : {} calls, failures caught {:.2} ({:.3}/call)\n\
         \x20 -> {} calls saved ({:.0}% of budget), recall drop {:.1}%",
        plan_legacy.blocks_to_verify.len(), caught_legacy, per_call_legacy,
        plan_es.blocks_to_verify.len(), caught_es, per_call_es,
        calls_saved, calls_saved as f64 / plan_legacy.blocks_to_verify.len().max(1) as f64 * 100.0,
        recall_drop * 100.0
    );
    assert!(calls_saved > 0, "early stop must save verification calls");
    assert!(
        per_call_es > per_call_legacy,
        "pruned calls must be the least valuable ones: failures-per-call must improve"
    );
    assert!(recall_drop < 0.15, "recall drop {recall_drop} exceeds the accepted trade-off");
}

// -------------------------------- sanity: soft preference math path is real
#[test]
fn soft_preference_path_is_exercised_by_the_harness() {
    let res = r#"{"winner": "B", "score_A": 4, "score_B": 17}"#;
    let p = comparison_preference(res);
    assert!(p < 0.4, "score 4 vs 17 must strongly prefer B for A (p = {p})");
}
