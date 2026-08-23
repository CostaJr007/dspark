//! Cost-Aware Scheduler module.
//! Optimizes API spending and latency by scheduling remote verifications only for high-risk blocks.

use super::confidence_head::{BlockConfidence, RiskLevel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationPlan {
    pub blocks_to_verify: Vec<usize>,
    pub total_blocks: usize,
    pub pruned_blocks_count: usize,
    pub estimated_cost: f64,
    pub expected_acceptance_rate: f64,
}

pub struct CostScheduler {
    pub max_api_calls: usize,
    pub cost_per_verification: f64,
}

impl CostScheduler {
    pub fn new(max_api_calls: usize, cost_per_verification: f64) -> Self {
        Self {
            max_api_calls,
            cost_per_verification,
        }
    }
}

impl Default for CostScheduler {
    fn default() -> Self {
        Self::new(20, 0.002) // Max 20 calls, ~$0.002 per deepseek call
    }
}

impl CostScheduler {
    /// Schedules blocks for remote verification using budget-aware pruning
    pub fn schedule_verification(&self, confidences: &[BlockConfidence]) -> VerificationPlan {
        let total_blocks = confidences.len();
        
        let mut candidates: Vec<(usize, f64, RiskLevel)> = confidences
            .iter()
            .filter(|b| b.needs_verification)
            .map(|b| (b.block_id, 1.0 - b.confidence_score, b.risk_level.clone()))
            .collect();

        // Sort descending by risk (highest risk first)
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut blocks_to_verify = Vec::new();
        let mut estimated_cost = 0.0;
        let mut expected_acceptance_sum = 0.0;

        for (block_id, _risk, risk_level) in candidates.iter().take(self.max_api_calls) {
            blocks_to_verify.push(*block_id);
            estimated_cost += self.cost_per_verification;

            let survival_prob = match risk_level {
                RiskLevel::High => 0.35,
                RiskLevel::Medium => 0.70,
                RiskLevel::Low => 0.95,
            };
            expected_acceptance_sum += survival_prob;
        }

        let verified_count = blocks_to_verify.len();
        let pruned_count = total_blocks.saturating_sub(verified_count);
        let acceptance_rate = if verified_count > 0 {
            expected_acceptance_sum / verified_count as f64
        } else {
            1.0
        };

        VerificationPlan {
            blocks_to_verify,
            total_blocks,
            pruned_blocks_count: pruned_count,
            estimated_cost,
            expected_acceptance_rate: acceptance_rate,
        }
    }
}
