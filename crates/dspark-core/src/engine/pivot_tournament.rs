//! Probabilistic Pivot Tournament (PPT) module.
//! Implements O(Nk) tournament algorithm from LLM-as-a-Verifier (Kwok et al., 2026).

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
    pub rankings: Vec<(usize, f64)>, // (trajectory_idx, win_mass)
    pub total_comparisons: usize,
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
        let (ring_results, ring_comps) = self.ring_pass(trajectories, criteria).await;

        // STAGE 2: Pivot Selection (top-k from ring pass)
        let pivots = self.select_pivots(&ring_results, k, n);

        // STAGE 3: Pivot Tournament (O(Nk) comparisons)
        let (rankings, tourney_comps) = self.pivot_tournament(trajectories, &pivots, criteria).await;

        // STAGE 4: Winner selection
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

    /// Stage 1: Ring Pass compares (0 vs 1, 1 vs 2, ..., n-1 vs 0)
    async fn ring_pass(
        &self,
        trajectories: &[DraftTrajectory],
        criteria: &str,
    ) -> (Vec<(usize, usize, bool)>, usize) {
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
                let a_won = !res.contains("\"winner\": \"B\"") && (res.contains("\"winner\": \"A\"") || res.contains("A is better"));
                (i, j, a_won)
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(res) = handle.await {
                results.push(res);
            }
        }

        let count = results.len();
        (results, count)
    }

    /// Stage 2: Select top-k pivots based on ring pass win mass
    fn select_pivots(&self, ring_results: &[(usize, usize, bool)], k: usize, total_n: usize) -> Vec<usize> {
        let mut wins: HashMap<usize, usize> = HashMap::new();

        for &(i, j, a_won) in ring_results {
            if a_won {
                *wins.entry(i).or_default() += 1;
            } else {
                *wins.entry(j).or_default() += 1;
            }
        }

        let mut ranked: Vec<(usize, usize)> = (0..total_n)
            .map(|idx| (idx, *wins.get(&idx).unwrap_or(&0)))
            .collect();
        ranked.sort_by_key(|a| std::cmp::Reverse(a.1));

        ranked.into_iter().take(k).map(|(idx, _)| idx).collect()
    }

    /// Stage 3: Pivot Tournament (Non-pivots vs Pivots + Pivots vs Pivots)
    async fn pivot_tournament(
        &self,
        trajectories: &[DraftTrajectory],
        pivots: &[usize],
        criteria: &str,
    ) -> (Vec<(usize, f64)>, usize) {
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
                    let a_won = !res.contains("\"winner\": \"B\"") && (res.contains("\"winner\": \"A\"") || res.contains("A is better"));
                    (i, p, a_won)
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
                    let a_won = !res.contains("\"winner\": \"B\"") && (res.contains("\"winner\": \"A\"") || res.contains("A is better"));
                    (p1, p2, a_won)
                }));
            }
        }

        let mut win_mass: HashMap<usize, f64> = HashMap::new();
        let mut matches_count: HashMap<usize, usize> = HashMap::new();
        let mut total_comps = 0;

        for handle in handles {
            if let Ok((i, j, a_won)) = handle.await {
                total_comps += 1;
                if a_won {
                    *win_mass.entry(i).or_default() += 1.0;
                } else {
                    *win_mass.entry(j).or_default() += 1.0;
                }
                *matches_count.entry(i).or_default() += 1;
                *matches_count.entry(j).or_default() += 1;
            }
        }

        let rankings: Vec<(usize, f64)> = (0..trajectories.len())
            .map(|idx| {
                let wins = *win_mass.get(&idx).unwrap_or(&0.0);
                let total = *matches_count.get(&idx).unwrap_or(&1).max(&1);
                (idx, wins / total as f64)
            })
            .collect();

        (rankings, total_comps)
    }
}
