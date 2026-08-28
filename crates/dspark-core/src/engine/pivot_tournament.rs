//! Probabilistic Pivot Tournament (PPT) module.
//! Implements the O(Nk) tournament algorithm from LLM-as-a-Verifier (Kwok et al., 2026),
//! including the Bradley-Terry soft updates derived from the verifier's continuous scores.
//!
//! Fidelity notes vs. the paper (arXiv:2607.05391, Algorithm 1):
//! - ring pass: every candidate appears exactly once in the "A" slot and once in the "B"
//!   slot (identity Hamiltonian cycle), cancelling the verifier's positional bias;
//! - soft updates: `w_i += p`, `w_j += 1 - p` with p = sigmoid(R_i - R_j) whenever the
//!   response carries 1-20 scores; binary winner parsing remains as fallback;
//! - pivot selection: top-k by ring-pass mean preference w_i/c_i;
//! - deviation (documented): ring pairs are NOT excluded from pivot rounds, so the
//!   comparison count stays exactly N + k(N-k) + C(k,2) (the paper's stated total),
//!   keeping the count deterministic and reproducible over the wire.

use crate::client::{ClientError, ModelClient};
use crate::utils::prompt_optimizer::PromptOptimizer;
use super::speculative_drafter::DraftTrajectory;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentResult {
    pub best_trajectory_idx: usize,
    pub rankings: Vec<(usize, f64)>, // (trajectory_idx, mean preference w_i/c_i)
    pub total_comparisons: usize,
}

impl TournamentResult {
    /// True when the top two win rates are within `epsilon`, i.e. the winner
    /// is statistically ambiguous and flagship arbitration may be warranted.
    pub fn is_tie(&self, epsilon: f64) -> bool {
        let mut rates: Vec<f64> = self.rankings.iter().map(|(_, r)| *r).collect();
        rates.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        match (rates.first(), rates.get(1)) {
            (Some(top), Some(second)) => top - second <= epsilon,
            _ => false,
        }
    }
}

/// Exact comparison count of the implemented PPT algorithm:
/// ring pass (N) + non-pivots vs pivots ((N-k)*k) + pivots vs pivots (C(k,2)).
/// Mirrors the runtime pivot clamp `k.clamp(1, (n / 2).max(1))`.
pub fn tournament_comparison_count(n: usize, k_requested: usize) -> usize {
    let k = k_requested.clamp(1, (n / 2).max(1));
    n + n.saturating_sub(k) * k + k * (k.saturating_sub(1)) / 2
}

/// Bradley-Terry preference of A over B: p = sigmoid(R_a - R_b).
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Parse a 1-20 score field (`"score_A": 15`) from a comparison response.
fn parse_score(res: &str, key: &str) -> Option<f64> {
    let marker = format!("\"{}\"", key);
    let idx = res.find(&marker)?;
    let tail = &res[idx + marker.len()..];
    let digits: String = tail
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let v: f64 = digits.parse().ok()?;
    (1.0..=20.0).contains(&v).then_some(v / 20.0)
}

/// Preference of candidate A (over B) for a single comparison response.
///
/// Prefers continuous scores (Bradley-Terry of normalized 1-20 rewards, per
/// Eq. 3.2 of the paper); falls back to the binary winner parse when scores
/// are absent (e.g. restricted APIs). EQUAL maps to 0.5.
/// Public for benchmarking harnesses (A/B selection-accuracy experiments).
pub fn comparison_preference(res: &str) -> f64 {
    if let (Some(ra), Some(rb)) = (parse_score(res, "score_A"), parse_score(res, "score_B")) {
        return sigmoid(ra - rb);
    }
    if res.contains("EQUAL") {
        return 0.5;
    }
    let a_won = !res.contains("\"winner\": \"B\"")
        && (res.contains("\"winner\": \"A\"") || res.contains("A is better"));
    if a_won {
        1.0
    } else if res.contains("\"winner\": \"B\"") || res.contains("B is better") {
        0.0
    } else {
        0.5
    }
}

pub struct PivotTournament {
    client: Arc<ModelClient>,
    n_pivots: usize,
    semaphore: Arc<Semaphore>,
    optimizer: PromptOptimizer,
}

impl PivotTournament {
    pub fn new(client: ModelClient, n_pivots: usize) -> Self {
        Self {
            client: Arc::new(client),
            n_pivots,
            semaphore: Arc::new(Semaphore::new(10)),
            optimizer: PromptOptimizer::new(),
        }
    }

    pub fn with_model(model_name: &str, n_pivots: usize) -> Result<Self, ClientError> {
        let client = ModelClient::from_spec(model_name)?;
        Ok(Self::new(client, n_pivots))
    }

    /// Executes the full PPT algorithm across all candidate trajectories
    pub async fn run_tournament(
        &self,
        trajectories: &[DraftTrajectory],
        criteria: &str,
    ) -> TournamentResult {
        let n = trajectories.len();
        if n == 0 {
            return TournamentResult {
                best_trajectory_idx: 0,
                rankings: vec![],
                total_comparisons: 0,
            };
        }
        if n == 1 {
            return TournamentResult {
                best_trajectory_idx: 0,
                rankings: vec![(0, 1.0)],
                total_comparisons: 0,
            };
        }

        let k = self.n_pivots.clamp(1, (n / 2).max(1));

        // STAGE 1: Ring Pass (Hamiltonian cycle adjacent comparisons)
        // Soft updates accumulate into ring-scoped accumulators for pivot selection.
        let (ring_mass, ring_count, ring_comps) = self.ring_pass(trajectories, criteria).await;

        // STAGE 2: Pivot Selection (top-k by ring-pass mean preference w_i/c_i)
        let pivots = self.select_pivots(&ring_mass, &ring_count, k, n);

        // STAGE 3: Pivot Tournament (O(Nk) comparisons), aggregated with ring totals
        let (win_mass, matches_count, tourney_comps) = self
            .pivot_tournament(trajectories, &pivots, criteria)
            .await;

        // Aggregate ring + pivot accumulators (soft preferences are additive)
        let mut final_mass = ring_mass;
        let mut final_count = ring_count;
        for (idx, mass) in win_mass {
            *final_mass.entry(idx).or_default() += mass;
        }
        for (idx, count) in matches_count {
            *final_count.entry(idx).or_default() += count;
        }

        let rankings: Vec<(usize, f64)> = (0..n)
            .map(|idx| {
                let mass = *final_mass.get(&idx).unwrap_or(&0.0);
                let count = final_count.get(&idx).copied().unwrap_or(1).max(1);
                (idx, mass / count as f64)
            })
            .collect();

        // STAGE 4: Winner selection (highest count-normalized preference)
        let best_idx = rankings
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(idx, _)| *idx)
            .unwrap_or(0);

        TournamentResult {
            best_trajectory_idx: best_idx,
            rankings,
            total_comparisons: ring_comps + tourney_comps,
        }
    }

    /// Stage 1: Ring Pass compares (0 vs 1, 1 vs 2, ..., n-1 vs 0) with soft updates.
    async fn ring_pass(
        &self,
        trajectories: &[DraftTrajectory],
        criteria: &str,
    ) -> (HashMap<usize, f64>, HashMap<usize, usize>, usize) {
        let n = trajectories.len();
        let mut handles = Vec::new();

        for i in 0..n {
            let j = (i + 1) % n;
            let client = Arc::clone(&self.client);
            let sem = Arc::clone(&self.semaphore);
            let prompt = self.optimizer.generate_comparison_prompt(
                &trajectories[i].code_blocks,
                &trajectories[j].code_blocks,
                criteria,
            );

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.ok();
                let res = client.complete(&prompt, None, 0.0, false).await.unwrap_or_default();
                (i, j, comparison_preference(&res))
            }));
        }

        let mut mass: HashMap<usize, f64> = HashMap::new();
        let mut count: HashMap<usize, usize> = HashMap::new();
        let mut comps = 0;
        for handle in handles {
            if let Ok((i, j, p)) = handle.await {
                comps += 1;
                *mass.entry(i).or_default() += p;
                *mass.entry(j).or_default() += 1.0 - p;
                *count.entry(i).or_default() += 1;
                *count.entry(j).or_default() += 1;
            }
        }
        (mass, count, comps)
    }

    /// Stage 2: Select top-k pivots by ring-pass mean preference w_i/c_i.
    fn select_pivots(
        &self,
        ring_mass: &HashMap<usize, f64>,
        ring_count: &HashMap<usize, usize>,
        k: usize,
        total_n: usize,
    ) -> Vec<usize> {
        let mut ranked: Vec<(usize, f64)> = (0..total_n)
            .map(|idx| {
                let mass = *ring_mass.get(&idx).unwrap_or(&0.0);
                let count = ring_count.get(&idx).copied().unwrap_or(1).max(1);
                (idx, mass / count as f64)
            })
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.into_iter().take(k).map(|(idx, _)| idx).collect()
    }

    /// Stage 3: Pivot Tournament (Non-pivots vs Pivots + Pivots vs Pivots)
    async fn pivot_tournament(
        &self,
        trajectories: &[DraftTrajectory],
        pivots: &[usize],
        criteria: &str,
    ) -> (HashMap<usize, f64>, HashMap<usize, usize>, usize) {
        let pivot_set: HashSet<usize> = pivots.iter().copied().collect();
        let mut handles = Vec::new();

        // 1. Non-pivots vs Pivots
        for i in 0..trajectories.len() {
            if pivot_set.contains(&i) {
                continue;
            }
            for &p in pivots {
                let client = Arc::clone(&self.client);
                let sem = Arc::clone(&self.semaphore);
                let prompt = self.optimizer.generate_comparison_prompt(
                    &trajectories[i].code_blocks,
                    &trajectories[p].code_blocks,
                    criteria,
                );

                handles.push(tokio::spawn(async move {
                    let _permit = sem.acquire().await.ok();
                    let res = client.complete(&prompt, None, 0.0, false).await.unwrap_or_default();
                    (i, p, comparison_preference(&res))
                }));
            }
        }

        // 2. Pivots vs Pivots
        for (p1_idx, &p1) in pivots.iter().enumerate() {
            for &p2 in pivots.iter().skip(p1_idx + 1) {
                let client = Arc::clone(&self.client);
                let sem = Arc::clone(&self.semaphore);
                let prompt = self.optimizer.generate_comparison_prompt(
                    &trajectories[p1].code_blocks,
                    &trajectories[p2].code_blocks,
                    criteria,
                );

                handles.push(tokio::spawn(async move {
                    let _permit = sem.acquire().await.ok();
                    let res = client.complete(&prompt, None, 0.0, false).await.unwrap_or_default();
                    (p1, p2, comparison_preference(&res))
                }));
            }
        }

        let mut win_mass: HashMap<usize, f64> = HashMap::new();
        let mut matches_count: HashMap<usize, usize> = HashMap::new();
        let mut total_comps = 0;

        for handle in handles {
            if let Ok((i, j, p)) = handle.await {
                total_comps += 1;
                *win_mass.entry(i).or_default() += p;
                *win_mass.entry(j).or_default() += 1.0 - p;
                *matches_count.entry(i).or_default() += 1;
                *matches_count.entry(j).or_default() += 1;
            }
        }

        (win_mass, matches_count, total_comps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_tie_detects_ambiguous_winners() {
        let clear = TournamentResult {
            best_trajectory_idx: 0,
            rankings: vec![(0, 0.9), (1, 0.4), (2, 0.3)],
            total_comparisons: 34,
        };
        assert!(!clear.is_tie(0.05));

        let tied = TournamentResult {
            best_trajectory_idx: 0,
            rankings: vec![(0, 0.51), (1, 0.49), (2, 0.2)],
            total_comparisons: 34,
        };
        assert!(tied.is_tie(0.05));
    }

    #[test]
    fn comparison_count_formula_matches_documented_small_cases() {
        // Requested k is clamped at runtime for small N; the formula mirrors it.
        assert_eq!(tournament_comparison_count(5, 2), 12);
        assert_eq!(tournament_comparison_count(10, 3), 34);
        assert_eq!(tournament_comparison_count(20, 3), 74);
        assert_eq!(tournament_comparison_count(3, 3), 5); // k clamped to 1
    }

    #[test]
    fn soft_preference_parses_scores_bradley_terry() {
        let res = r#"{"winner": "A", "score_A": 15, "score_B": 5}"#;
        let p = comparison_preference(res);
        assert!(p > 0.6, "score 15 vs 5 must strongly prefer A, got {p}");
        assert!(p < 1.0, "soft preference is never hard 1.0");

        // Symmetric inverse
        let res2 = r#"{"winner": "B", "score_A": 5, "score_B": 15}"#;
        let p2 = comparison_preference(res2);
        assert!((p + p2 - 1.0).abs() < 1e-9, "preferences must be complementary");

        // Equal scores -> 0.5
        let res3 = r#"{"winner": "EQUAL", "score_A": 10, "score_B": 10}"#;
        assert!((comparison_preference(res3) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn binary_fallback_without_scores() {
        assert_eq!(comparison_preference("{\"winner\": \"A\"}"), 1.0);
        assert_eq!(comparison_preference("{\"winner\": \"B\"}"), 0.0);
        assert_eq!(comparison_preference("{\"winner\": \"EQUAL\"}"), 0.5);
        assert_eq!(comparison_preference(""), 0.5);
    }

    #[tokio::test]
    async fn soft_updates_yield_fractional_rankings() {
        use crate::client::ScriptedClient;

        let client = ModelClient::Scripted(ScriptedClient::new("judge-x", |_| {
            "{\"winner\": \"B\", \"score_A\": 5, \"score_B\": 18}".to_string()
        }));
        let tournament = PivotTournament::new(client, 2);
        let trajectories: Vec<DraftTrajectory> = (0..5)
            .map(|i| DraftTrajectory {
                id: i,
                full_code: format!("fn c{}() {{ {} }}", i, i),
                code_blocks: vec![crate::utils::ast_resolver::CodeBlock {
                    function_name: format!("c{}", i),
                    code: format!("fn c{}() {{ {} }}", i, i),
                    line_count: 1,
                }],
                confidence_score: 0.8,
                ast_valid: true,
            })
            .collect();

        let res = tournament.run_tournament(&trajectories, "Check correctness").await;

        assert_eq!(res.total_comparisons, tournament_comparison_count(5, 2));
        assert_eq!(res.rankings.len(), 5);
        for (_, rate) in &res.rankings {
            assert!(
                *rate > 0.0 && *rate < 1.0,
                "soft updates must yield fractional preferences, got {rate}"
            );
        }
    }
}
