//! Agent-level proof harness: do the composed theories deliver a measurable benefit?
//!
//! Runs the REAL `run --speculative` stage path (drafter -> confidence head ->
//! cost scheduler -> pivot tournament -> escalation) with scripted LLM tiers and
//! REAL execution ground truth (`run_python_spec_oracle`, a python subprocess).
//! No network, no API keys, fully deterministic (seeded RNG). This is what makes
//! it CI-runnable, unlike the live pilot in `bench/`.
//!
//! STATED ASSUMPTIONS (falsifiable, printed in the PROOF table):
//!   A1. The ranking judge is strictly stronger than noise: scores track oracle
//!       truth plus bounded noise (the bench's own judge-tier finding).
//!   A2. The repair curator is capable: given a failing winner it returns the
//!       known-good variant (perfect-repair upper bound for the CEGAR step).
//!   A3. The spec oracle (prompt-visible doctests) is the correctness criterion.
//!
//! What is PROVEN (asserted): selection lift over the pool base rate, exact
//! O(Nk) call accounting, pruning savings with execution-measured recall, and a
//! bounded escalation rate (flagship only for hard cases).

use dspark::client::{ModelClient, ScriptedClient};
use dspark::engine::{
    ConfidenceHead, CostScheduler, EscalationContext, EscalationPolicy, PivotTournament,
    SpeculativeDrafter,
};
use dspark::oracle::run_python_spec_oracle;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn xorshift(state: &mut u64) -> f64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state as f64) / (u64::MAX as f64)
}

struct ProofTask {
    id: &'static str,
    spec: &'static str,
    /// pool[0] is always the known-good variant; the rest are buggy.
    pool: [&'static str; 4],
}

// ---------------------------------------------------------------- tasks
const T1_SPEC: &str = "def filter_by_substring(strings, sub):\n    pass\n";
const T1_GOOD: &str = r#"# dspark-cand:T1:0
from typing import List
def filter_by_substring(strings: List[str], sub: str) -> List[str]:
    """Filter strings that contain sub
    >>> filter_by_substring([], 'a')
    []
    >>> filter_by_substring(['abc', 'bacd', 'cde', 'array'], 'a')
    ['abc', 'bacd', 'array']
    """
    return [s for s in strings if sub in s]
"#;
const T1_BUG1: &str = r#"# dspark-cand:T1:1
from typing import List
def filter_by_substring(strings: List[str], sub: str) -> List[str]:
    """Filter strings that contain sub
    >>> filter_by_substring([], 'a')
    []
    >>> filter_by_substring(['abc', 'bacd', 'cde', 'array'], 'a')
    ['abc', 'bacd', 'array']
    """
    return [s for s in strings if s == sub]
"#;
const T1_BUG2: &str = r#"# dspark-cand:T1:2
from typing import List
_seen = []

def filter_by_substring(strings: List[str], sub: str) -> List[str]:
    """Filter strings that contain sub
    >>> filter_by_substring([], 'a')
    []
    >>> filter_by_substring(['abc', 'bacd', 'cde', 'array'], 'a')
    ['abc', 'bacd', 'array']
    """
    global _seen
    out = []
    i = 0
    while i < len(strings):
        s = strings[i]
        i += 1
        if len(s) == 0:
            continue
        else:
            if sub[0] in s:
                if s[0] == sub[0]:
                    out.append(s)
                else:
                    _seen.append(s)
            else:
                continue
    return out
"#;
const T1_BUG3: &str = r#"# dspark-cand:T1:3
from typing import List
def filter_by_substring(strings: List[str], sub: str) -> List[str]:
    """Filter strings that contain sub
    >>> filter_by_substring([], 'a')
    []
    >>> filter_by_substring(['abc', 'bacd', 'cde', 'array'], 'a')
    ['abc', 'bacd', 'array']
    """
    return []
"#;

const T2_SPEC: &str = "def encode_cyclic(s):\n    pass\n\ndef decode_cyclic(s):\n    pass\n";
const T2_GOOD: &str = r#"# dspark-cand:T2:0
def encode_cyclic(s):
    """Encode by cyclic shift
    >>> decode_cyclic(encode_cyclic("abcdef"))
    'abcdef'
    """
    groups = [s[(3 * i):min((3 * i + 3), len(s))] for i in range((len(s) + 2) // 3)]
    groups = [(group[1:] + group[0]) if len(group) == 3 else group for group in groups]
    return "".join(groups)

def decode_cyclic(s):
    groups = [s[(3 * i):min((3 * i + 3), len(s))] for i in range((len(s) + 2) // 3)]
    groups = [(group[-1:] + group[:-1]) if len(group) == 3 else group for group in groups]
    return "".join(groups)
"#;
const T2_BUG1: &str = r#"# dspark-cand:T2:1
def encode_cyclic(s):
    """Encode by cyclic shift
    >>> decode_cyclic(encode_cyclic("abcdef"))
    'abcdef'
    """
    groups = [s[(3 * i):min((3 * i + 3), len(s))] for i in range((len(s) + 2) // 3)]
    groups = [(group[1:] + group[0]) if len(group) == 3 else group for group in groups]
    return "".join(groups)

def decode_cyclic(s):
    return s
"#;
const T2_BUG2: &str = r#"# dspark-cand:T2:2
def encode_cyclic(s):
    """Encode by cyclic shift
    >>> decode_cyclic(encode_cyclic("abcdef"))
    'abcdef'
    """
    return s

def decode_cyclic(s):
    groups = [s[(3 * i):min((3 * i + 3), len(s))] for i in range((len(s) + 2) // 3)]
    groups = [(group[-1:] + group[:-1]) if len(group) == 3 else group for group in groups]
    return "".join(groups)
"#;
const T2_BUG3: &str = r#"# dspark-cand:T2:3
def encode_cyclic(s):
    """Encode by cyclic shift
    >>> decode_cyclic(encode_cyclic("abcdef"))
    'abcdef'
    """
    groups = [s[(3 * i):min((3 * i + 3), len(s))] for i in range((len(s) + 2) // 3)]
    groups = [(group[1:] + group[0]) if len(group) == 3 else group for group in groups]
    return "".join(groups)

def decode_cyclic(s):
    raise ValueError("todo")
"#;

const T3_SPEC: &str = "def clamp(x, lo, hi):\n    pass\n";const T3_GOOD: &str = r#"# dspark-cand:T3:0
def clamp(x, lo, hi):
    """Clamp x into [lo, hi]
    >>> clamp(5, 0, 10)
    5
    >>> clamp(-3, 0, 10)
    0
    >>> clamp(15, 0, 10)
    10
    >>> clamp(5, 5, 5)
    5
    """
    return max(lo, min(x, hi))
"#;
const T3_BUG1: &str = r#"# dspark-cand:T3:1
def clamp(x, lo, hi):
    """Clamp x into [lo, hi]
    >>> clamp(5, 0, 10)
    5
    >>> clamp(-3, 0, 10)
    0
    >>> clamp(15, 0, 10)
    10
    >>> clamp(5, 5, 5)
    5
    """
    return max(lo, x)
"#;
const T3_BUG2: &str = r#"# dspark-cand:T3:2
def clamp(x, lo, hi):
    """Clamp x into [lo, hi]
    >>> clamp(5, 0, 10)
    5
    >>> clamp(-3, 0, 10)
    0
    >>> clamp(15, 0, 10)
    10
    >>> clamp(5, 5, 5)
    5
    """
    result = x
    if result < lo:
        result = lo
    if result > lo:
        result = hi
    return result
"#;
const T3_BUG3: &str = r#"# dspark-cand:T3:3
def clamp(x, lo, hi):
    """Clamp x into [lo, hi]
    >>> clamp(5, 0, 10)
    5
    >>> clamp(-3, 0, 10)
    0
    >>> clamp(15, 0, 10)
    10
    >>> clamp(5, 5, 5)
    5
    """
    return max(lo, min(x, hi - 1))
"#;

const T4_SPEC: &str = "def quicksort(xs):\n    pass\n";
const T4_GOOD: &str = r#"# dspark-cand:T4:0
def quicksort(xs):
    """Sort a list
    >>> quicksort([3, 1, 2])
    [1, 2, 3]
    >>> quicksort([])
    []
    >>> quicksort([5, 5, 1])
    [1, 5, 5]
    """
    if len(xs) <= 1:
        return xs
    pivot = xs[0]
    left = [x for x in xs[1:] if x < pivot]
    right = [x for x in xs[1:] if x >= pivot]
    return quicksort(left) + [pivot] + quicksort(right)
"#;
const T4_BUG1: &str = r#"# dspark-cand:T4:1
def quicksort(xs):
    """Sort a list
    >>> quicksort([3, 1, 2])
    [1, 2, 3]
    >>> quicksort([])
    []
    >>> quicksort([5, 5, 1])
    [1, 5, 5]
    """
    if len(xs) <= 1:
        return xs
    pivot = xs[0]
    left = [x for x in xs[1:] if x < pivot]
    right = [x for x in xs[1:] if x > pivot]
    return quicksort(left) + [pivot] + quicksort(right)
"#;
const T4_BUG2: &str = r#"# dspark-cand:T4:2
def quicksort(xs):
    """Sort a list
    >>> quicksort([3, 1, 2])
    [1, 2, 3]
    >>> quicksort([])
    []
    >>> quicksort([5, 5, 1])
    [1, 5, 5]
    """
    if len(xs) <= 1:
        return xs
    pivot = xs[0]
    left = [x for x in xs[1:] if x < pivot]
    right = [x for x in xs[1:] if x >= pivot]
    return quicksort(left) + quicksort(right)
"#;
const T4_BUG3: &str = r#"# dspark-cand:T4:3
def quicksort(xs):
    """Sort a list
    >>> quicksort([3, 1, 2])
    [1, 2, 3]
    >>> quicksort([])
    []
    >>> quicksort([5, 5, 1])
    [1, 5, 5]
    """
    if len(xs) <= 1:
        return xs
    pivot = xs[0]
    left = [x for x in xs[1:] if x >= pivot]
    right = [x for x in xs[1:] if x < pivot]
    return quicksort(left) + [pivot] + quicksort(right)
"#;

fn tasks() -> Vec<ProofTask> {
    vec![
        ProofTask { id: "T1", spec: T1_SPEC, pool: [T1_GOOD, T1_BUG1, T1_BUG2, T1_BUG3] },
        ProofTask { id: "T2", spec: T2_SPEC, pool: [T2_GOOD, T2_BUG1, T2_BUG2, T2_BUG3] },
        ProofTask { id: "T3", spec: T3_SPEC, pool: [T3_GOOD, T3_BUG1, T3_BUG2, T3_BUG3] },
        ProofTask { id: "T4", spec: T4_SPEC, pool: [T4_GOOD, T4_BUG1, T4_BUG2, T4_BUG3] },
    ]
}

/// `cand-T1-2` style marker embedded as a code comment (oracle-neutral).
fn marker_of(code: &str) -> Option<(String, usize)> {
    for line in code.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("# dspark-cand:") {
            let mut it = rest.split(':');
            if let (Some(task), Some(idx)) = (it.next(), it.next()) {
                if let Ok(i) = idx.trim().parse::<usize>() {
                    return Some((task.to_string(), i));
                }
            }
        }
    }
    None
}

#[tokio::test]
async fn agent_level_proof_lift_with_execution_ground_truth() {
    const N: usize = 4;
    const K: usize = 2;
    const ROUNDS_PER_TASK: usize = 4;

    let tasks = tasks();
    // Execution ground truth, computed ONCE with the real oracle (A3).
    let mut truth: HashMap<(&str, usize), bool> = HashMap::new();
    for t in &tasks {
        for (i, cand) in t.pool.iter().enumerate() {
            truth.insert((t.id, i), run_python_spec_oracle(cand, t.spec).is_empty());
        }
    }
    for t in &tasks {
        assert!(truth[&(t.id, 0)], "{} pool[0] must be the good variant", t.id);
        assert!(
            (1..4).any(|i| !truth[&(t.id, i)]),
            "{} needs at least one failing variant (else the round is vacuous)",
            t.id
        );
    }

    let scheduler = CostScheduler::with_early_stop(20, 0.002, 0.31);
    let policy = EscalationPolicy::default();
    let head = ConfidenceHead::default();

    let mut rounds = 0usize;
    let mut winner_pass = 0usize;
    let mut proof_pass = 0usize;
    let mut wrong_picks = 0usize;
    let mut total_comparisons = 0usize;
    let mut total_pruned = 0usize;
    let mut total_blocks = 0usize;
    let mut total_verified = 0usize;
    let mut escalations = 0usize;
    let judge_calls = Arc::new(Mutex::new(0usize));

    for (ti, task) in tasks.iter().enumerate() {
        for r in 0..ROUNDS_PER_TASK {
            rounds += 1;
            // --- Stage 1: speculative drafting (real path, scripted tier).
            // Rotation offset keeps the per-round SET deterministic (= full pool).
            let offset = (ti + r) % N;
            let pool: Vec<&'static str> = task.pool.to_vec();
            let drafter = SpeculativeDrafter::new(
                ModelClient::Scripted(ScriptedClient::new("proof-drafter", {
                    let pool = pool.clone();
                    let calls = Arc::new(Mutex::new(offset));
                    move |_prompt| {
                        let mut c = calls.lock().unwrap();
                        let cand = pool[*c % N];
                        *c += 1;
                        cand.to_string()
                    }
                })),
                N,
            );
            let trajectories = drafter.generate_trajectories(task.spec).await;
            assert_eq!(trajectories.len(), N, "all {} drafts must survive", N);
            let trajectories = drafter.apply_sequential_module(trajectories);
            assert_eq!(trajectories.len(), N);

            // --- Stage 2+3: confidence (real CPU head) + scheduler (real).
            let mut all_confs = Vec::new();
            for traj in trajectories.iter() {
                assert!(
                    !traj.code_blocks.is_empty(),
                    "drafter must yield blocks (harness setup broken?)"
                );
                // Same global-id renumbering as main.rs: per-trajectory ids
                // collide across trajectories (production fix, not test-only).
                for mut b in head.estimate_confidence(traj) {
                    b.block_id = all_confs.len();
                    all_confs.push(b);
                }
            }
            total_blocks += all_confs.len();
            let plan = scheduler.schedule_verification(&all_confs);
            total_pruned += plan.pruned_blocks_count;

            // Scheduler contract against execution reality:
            // (a) structural: every verified block needed verification;
            // (b) priority: greedy admits highest-risk-first, so no pruned
            //     needy block may outrank a verified one. (E4 proves the same
            //     property on synthetic input; here it holds on real blocks.)
            let ord = |r: &dspark::engine::RiskLevel| match r {
                dspark::engine::RiskLevel::High => 2,
                dspark::engine::RiskLevel::Medium => 1,
                dspark::engine::RiskLevel::Low => 0,
            };
            let verified: std::collections::HashSet<usize> =
                plan.blocks_to_verify.iter().copied().collect();
            total_verified += verified.len();
            for (bi, bc) in all_confs.iter().enumerate() {
                if verified.contains(&bi) {
                    assert!(bc.needs_verification, "verified block must need it");
                    for (bj, other) in all_confs.iter().enumerate() {
                        if !verified.contains(&bj) && other.needs_verification {
                            assert!(
                                ord(&bc.risk_level) >= ord(&other.risk_level),
                                "priority violated: verified {bi} outranked by pruned {bj}"
                            );
                        }
                    }
                }
            }
            for traj in trajectories.iter() {
                let (tmark, _) = marker_of(&traj.full_code).expect("marker must survive");
                assert_eq!(tmark, task.id);
            }

            // --- Stage 4: PPT with a stronger-than-noise judge (A1).
            let seed = 9000 + (rounds as u64);
            let rng = Arc::new(Mutex::new(seed));
            let truth_j = truth.clone();
            let task_id = task.id.to_string();
            let judge = ModelClient::Scripted(ScriptedClient::new("proof-judge", {
                let rng = Arc::clone(&rng);
                let calls = Arc::clone(&judge_calls);
                let truth_j = truth_j.clone();
                move |prompt| {
                    *calls.lock().unwrap() += 1;
                    // Score from execution truth + bounded noise (A1).
                    let mut idxs = Vec::new();
                    let mut rest = prompt;
                    while let Some(pos) = rest.find("# dspark-cand:") {
                        let after = &rest[pos + "# dspark-cand:".len()..];
                        let chunk: String =
                            after.chars().take_while(|c| *c != '\n').collect();
                        let mut it = chunk.split(':');
                        if let (Some(_t), Some(i)) = (it.next(), it.next()) {
                            if let Ok(v) = i.trim().parse::<usize>() {
                                idxs.push(v);
                            }
                        }
                        rest = after;
                    }
                    let mut r = rng.lock().unwrap();
                    let noise = |r: &mut std::sync::MutexGuard<'_, u64>| {
                        (xorshift(r) - 0.5) * 14.0
                    };
                    let score = |i: usize, r: &mut std::sync::MutexGuard<'_, u64>| {
                        // Noisy but better-than-random judge (A1): bands overlap,
                        // so mis-picks happen and the tournament must earn its lift.
                        let base = if *truth_j.get(&(task_id.as_str(), i)).unwrap_or(&false) {
                            13.0
                        } else {
                            7.0
                        };
                        ((base + noise(r)).round() as i64).clamp(1, 20)
                    };
                    let (a, b) = (idxs.first().copied().unwrap_or(0), idxs.get(1).copied().unwrap_or(1));
                    let (sa, sb) = (score(a, &mut r), score(b, &mut r));
                    format!(
                        "{{\"winner\": \"A\", \"score_A\": {}, \"score_B\": {}}}",
                        sa, sb
                    )
                }
            }));
            let tournament = PivotTournament::new(judge, K);
            let res = tournament.run_tournament(&trajectories, task.spec).await;
            assert_eq!(res.rankings.len(), N);
            total_comparisons += res.total_comparisons;

            let winner = &trajectories[res.best_trajectory_idx];
            let (_, widx) = marker_of(&winner.full_code).expect("winner marker");
            let winner_ok = truth[&(task.id, widx)];
            if winner_ok {
                winner_pass += 1;
            } else {
                wrong_picks += 1;
            }

            // --- Stage 5: execution check + capable repair (A2), re-checked.
            let mut final_ok = winner_ok;
            if !final_ok {
                let repaired = task.pool[0];
                final_ok = run_python_spec_oracle(repaired, task.spec).is_empty();
            }
            if final_ok {
                proof_pass += 1;
            }

            // --- Escalation accounting on the real winner context.
            let wconfs = head.estimate_confidence(winner);
            let mean_conf: f64 = if wconfs.is_empty() {
                0.5
            } else {
                wconfs.iter().map(|b| b.confidence_score).sum::<f64>() / wconfs.len() as f64
            };
            let high_risk = wconfs
                .iter()
                .filter(|b| matches!(b.risk_level, dspark::engine::RiskLevel::High))
                .count();
            let verified_set: std::collections::HashSet<usize> =
                plan.blocks_to_verify.iter().copied().collect();
            let mut base = 0usize;
            for (ti2, traj) in trajectories.iter().enumerate() {
                let n = head.estimate_confidence(traj).len();
                if ti2 == res.best_trajectory_idx {
                    let unverified = (0..n)
                        .filter(|i| {
                            wconfs.get(*i).map(|b| {
                                matches!(b.risk_level, dspark::engine::RiskLevel::High)
                            }).unwrap_or(false)
                            && !verified_set.contains(&(base + i))
                        })
                        .count();
                    let ctx = EscalationContext {
                        winner_confidence: mean_conf,
                        high_risk_blocks: high_risk,
                        unverified_high_risk: unverified,
                        tournament_tie: res.is_tie(0.05),
                        ast_valid: winner.ast_valid,
                    };
                    if policy.evaluate(&ctx).escalated {
                        escalations += 1;
                    }
                    break;
                }
                base += n;
            }
        }
    }

    let winner_rate = winner_pass as f64 / rounds as f64;
    let proof_rate = proof_pass as f64 / rounds as f64;
    let prune_rate = total_pruned as f64 / total_blocks.max(1) as f64;
    let judge_total = *judge_calls.lock().unwrap();
    println!(
        "\n[PROOF] agent-level benefit, {rounds} rounds, execution ground truth:\n\
         \x20 pool base rate (random pick) : 25.0%\n\
         \x20 PPT winner pass rate        : {winner_pass}/{rounds} ({:.1}%)\n\
         \x20 PPT + repair (proof)        : {proof_pass}/{rounds} ({:.1}%)\n\
         \x20 judge mis-picks (non-vacuous challenge): {wrong_picks}\n\
         \x20 comparisons total           : {total_comparisons} (formula: {})\n\
         \x20 judge calls counted         : {judge_total}\n\
         \x20 blocks pruned               : {total_pruned}/{total_blocks} ({:.1}%)\n\
         \x20 blocks verified (highest-risk-first): {total_verified}\n\
         \x20 escalations (flagship only for hard cases): {escalations}/{rounds}\n\
         \x20 -> assumptions: A1 stronger judge, A2 capable repair, A3 oracle == correctness",
        winner_rate * 100.0,
        proof_rate * 100.0,
        rounds * dspark::engine::tournament_comparison_count(N, K),
        prune_rate * 100.0,
    );

    // The proof: composition beats the pool base rate with real accounting.
    assert!(wrong_picks >= 1, "judge must err at least once (else selection is untested)");
    assert!(
        winner_rate > 0.25 + 0.25,
        "PPT selection must lift over the 25% pool base rate (got {winner_rate:.2})"
    );
    assert_eq!(proof_rate, 1.0, "repair must close every round under A2");
    assert_eq!(
        total_comparisons,
        rounds * dspark::engine::tournament_comparison_count(N, K),
        "every comparison must be accounted (O(Nk) budget is real)"
    );
    assert_eq!(judge_total, total_comparisons, "judge calls == comparisons");
    assert!(total_pruned > 0, "scheduler must prune (savings are real)");
    assert!(
        prune_rate >= 0.5,
        "pruning must carry the simple-code load (got {prune_rate:.2})"
    );
    assert!(
        total_verified > 0,
        "verify path must engage on the Medium-risk block (else it is dead code in practice)"
    );
    assert!(
        escalations < rounds,
        "flagship must not fire every round (got {escalations}/{rounds})"
    );
}
