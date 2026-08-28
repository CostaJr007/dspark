//! Cost-Aware Scheduler module.
//! Optimizes API spending and latency by scheduling remote verifications only for
//! high-risk blocks, following the greedy admission structure of the DSpark
//! hardware-aware prefix scheduler (arXiv:2607.05147, Algorithm 1) adapted from
//! batch-capacity throughput (tau * SPS(B)) to per-call cost.
//!
//! Non-anticipation invariant (Appendix A of the DSpark paper): the admission of a
//! block must depend only on information available before that block is verified.
//! All confidence inputs are computed from the draft before any verification runs,
//! and the early-stop decision uses only the prefix of the sorted candidate order,
//! so pruning cannot leak future verification outcomes into admission decisions.

use super::confidence_head::{BlockConfidence, RiskLevel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationPlan {
    pub blocks_to_verify: Vec<usize>,
    pub total_blocks: usize,
    pub pruned_blocks_count: usize,
    pub estimated_cost: f64,
    pub expected_acceptance_rate: f64,
    /// Expected number of verified-and-accepted blocks (sum of survival probabilities
    /// of the admitted candidates, analog of the accepted-length tau).
    pub expected_accepted: f64,
}

/// Per-risk-level survival probability used when a calibrated confidence head
/// (STS) is unavailable. Kept in one place so calibration can replace it later.
fn survival_probability(confidence: f64, risk_level: &RiskLevel) -> f64 {
    match risk_level {
        RiskLevel::High => confidence.clamp(0.0, 0.35),
        RiskLevel::Medium => confidence.clamp(0.0, 0.70),
        RiskLevel::Low => confidence.clamp(0.0, 0.95),
    }
}

pub struct CostScheduler {
    pub max_api_calls: usize,
    pub cost_per_verification: f64,
    /// Greedy admission stops when the marginal expected gain of verifying a block
    /// (its rejection risk, 1 - survival) falls below this threshold. Zero disables
    /// the early stop (mirrors the "verify everything until budget" baseline).
    pub min_marginal_expected_gain: f64,
}

impl CostScheduler {
    pub fn new(max_api_calls: usize, cost_per_verification: f64) -> Self {
        Self {
            max_api_calls,
            cost_per_verification,
            min_marginal_expected_gain: 0.0,
        }
    }

    pub fn with_early_stop(
        max_api_calls: usize,
        cost_per_verification: f64,
        min_marginal_expected_gain: f64,
    ) -> Self {
        Self {
            max_api_calls,
            cost_per_verification,
            min_marginal_expected_gain,
        }
    }
}

impl Default for CostScheduler {
    fn default() -> Self {
        Self::new(20, 0.002) // Max 20 calls, ~$0.002 per deepseek call
    }
}

impl CostScheduler {
    /// Schedules blocks for remote verification using budget-aware greedy admission.
    ///
    /// Candidates are sorted by rejection risk (1 - survival) descending; blocks are
    /// admitted greedily while (a) the budget remains and (b) the marginal expected
    /// gain clears `min_marginal_expected_gain` (early stop, non-anticipating because
    /// it only inspects the prefix of the sorted order).
    pub fn schedule_verification(&self, confidences: &[BlockConfidence]) -> VerificationPlan {
        let total_blocks = confidences.len();

        let mut candidates: Vec<(usize, f64, f64)> = confidences
            .iter()
            .filter(|b| b.needs_verification)
            .map(|b| {
                let survival = survival_probability(b.confidence_score, &b.risk_level);
                (b.block_id, 1.0 - survival, survival)
            })
            .collect();

        // Sort descending by risk (highest risk first)
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut blocks_to_verify = Vec::new();
        let mut estimated_cost = 0.0;
        let mut expected_acceptance_sum = 0.0;

        for (block_id, risk, survival) in candidates.iter() {
            if blocks_to_verify.len() >= self.max_api_calls {
                break;
            }
            // Early stop: stop admitting once the expected gain is too small.
            // Depends only on the current candidate (prefix of the sorted order).
            if *risk < self.min_marginal_expected_gain {
                break;
            }
            blocks_to_verify.push(*block_id);
            estimated_cost += self.cost_per_verification;
            expected_acceptance_sum += *survival;
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
            expected_accepted: expected_acceptance_sum,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(id: usize, confidence: f64, risk: RiskLevel) -> BlockConfidence {
        BlockConfidence {
            block_id: id,
            function_name: format!("fn_{id}"),
            confidence_score: confidence,
            needs_verification: true,
            risk_level: risk,
        }
    }

    #[test]
    fn early_stop_drops_low_marginal_gain_blocks() {
        let scheduler = CostScheduler::with_early_stop(10, 0.002, 0.5);
        let confs = vec![
            block(0, 0.30, RiskLevel::High),   // marginal 0.70 -> admitted
            block(1, 0.40, RiskLevel::High),   // marginal 0.60 -> admitted
            block(2, 0.90, RiskLevel::Low),    // marginal 0.05 -> early stop
            block(3, 0.10, RiskLevel::High),   // marginal 0.90 -> admitted first
        ];
        let plan = scheduler.schedule_verification(&confs);
        assert_eq!(plan.blocks_to_verify, vec![3, 0, 1]);
        assert!(plan.expected_accepted > 0.0 && plan.expected_accepted < 3.0);
    }

    #[test]
    fn zero_threshold_keeps_legacy_budget_cap_behavior() {
        let scheduler = CostScheduler::new(2, 0.002);
        let confs = vec![
            block(1, 0.40, RiskLevel::High),
            block(2, 0.30, RiskLevel::High),
            block(3, 0.70, RiskLevel::Medium),
        ];
        let plan = scheduler.schedule_verification(&confs);
        assert_eq!(plan.blocks_to_verify.len(), 2);
        assert!(plan.estimated_cost <= 0.004 + 1e-6);
    }

    #[test]
    fn admission_is_non_anticipating_in_block_order() {
        // Admission depends only on confidences (available pre-verification); the
        // early stop inspects only the sorted-order prefix. Changing a later block's
        // confidence must not change the relative admission order of earlier blocks.
        let scheduler = CostScheduler::with_early_stop(10, 0.002, 0.25);
        let base = vec![block(0, 0.30, RiskLevel::High), block(1, 0.60, RiskLevel::Medium)];
        let mut later_changed = base.clone();
        later_changed.push(block(2, 0.05, RiskLevel::High)); // riskier but later
        let plan_base = scheduler.schedule_verification(&base);
        let plan_later = scheduler.schedule_verification(&later_changed);
        assert_eq!(plan_base.blocks_to_verify, vec![0, 1]);
        assert_eq!(plan_later.blocks_to_verify, vec![2, 0, 1]);
        // The relative order among {0, 1} is preserved: 1 never admitted before 0.
        let pos0 = plan_later.blocks_to_verify.iter().position(|&b| b == 0).unwrap();
        let pos1 = plan_later.blocks_to_verify.iter().position(|&b| b == 1).unwrap();
        assert!(pos0 < pos1);
    }
}
