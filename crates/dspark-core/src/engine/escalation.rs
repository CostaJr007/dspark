//! Flagship escalation policy.
//!
//! Decides WHEN the expensive flagship curator must intervene on the residual
//! hard cases that survive the speculative pipeline (tournament ties, unverified
//! high-risk blocks, low winner confidence, broken AST). Everything else stays
//! on the cheap tier, keeping the verification budget concentrated where the
//! flagship actually outperforms.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscalationReason {
    /// Winner trajectory failed AST/dependency validation.
    AstInvalid,
    /// Top two trajectories are statistically indistinguishable.
    TournamentTie,
    /// Winner mean confidence fell below the policy floor.
    LowWinnerConfidence,
    /// High-risk winner blocks were pruned from remote verification.
    HighRiskUnverified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationDecision {
    pub escalated: bool,
    pub reasons: Vec<EscalationReason>,
}

#[derive(Debug, Clone)]
pub struct EscalationContext {
    /// Mean block confidence of the winning trajectory (0.0..=1.0).
    pub winner_confidence: f64,
    /// Number of high-risk blocks detected in the winner.
    pub high_risk_blocks: usize,
    /// How many of those high-risk blocks were pruned by the cost scheduler.
    pub unverified_high_risk: usize,
    /// Whether the PPT tournament produced an ambiguous winner.
    pub tournament_tie: bool,
    /// Whether the winner passed AST dependency validation.
    pub ast_valid: bool,
}

pub struct EscalationPolicy {
    /// Winner confidence below this floor triggers escalation.
    pub confidence_floor: f64,
    /// Hard cap on flagship invocations per orchestration run (0 disables escalation).
    pub max_flagship_calls_per_run: usize,
}

impl Default for EscalationPolicy {
    fn default() -> Self {
        Self {
            confidence_floor: 0.65,
            max_flagship_calls_per_run: 3,
        }
    }
}

impl EscalationPolicy {
    pub fn new(confidence_floor: f64, max_flagship_calls_per_run: usize) -> Self {
        Self {
            confidence_floor,
            max_flagship_calls_per_run,
        }
    }

    /// Pure evaluation of the escalation rules for the current run context.
    pub fn evaluate(&self, ctx: &EscalationContext) -> EscalationDecision {
        if self.max_flagship_calls_per_run == 0 {
            return EscalationDecision {
                escalated: false,
                reasons: Vec::new(),
            };
        }

        let mut reasons = Vec::new();
        if !ctx.ast_valid {
            reasons.push(EscalationReason::AstInvalid);
        }
        if ctx.tournament_tie {
            reasons.push(EscalationReason::TournamentTie);
        }
        if ctx.winner_confidence < self.confidence_floor {
            reasons.push(EscalationReason::LowWinnerConfidence);
        }
        if ctx.unverified_high_risk > 0 {
            reasons.push(EscalationReason::HighRiskUnverified);
        }

        EscalationDecision {
            escalated: !reasons.is_empty(),
            reasons,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn healthy() -> EscalationContext {
        EscalationContext {
            winner_confidence: 0.95,
            high_risk_blocks: 0,
            unverified_high_risk: 0,
            tournament_tie: false,
            ast_valid: true,
        }
    }

    #[test]
    fn healthy_winner_is_not_escalated() {
        let policy = EscalationPolicy::default();
        assert!(!policy.evaluate(&healthy()).escalated);
    }

    #[test]
    fn tie_escalates() {
        let policy = EscalationPolicy::default();
        let mut ctx = healthy();
        ctx.tournament_tie = true;
        let d = policy.evaluate(&ctx);
        assert!(d.escalated);
        assert_eq!(d.reasons, vec![EscalationReason::TournamentTie]);
    }

    #[test]
    fn low_confidence_and_unverified_high_risk_escalate_together() {
        let policy = EscalationPolicy::default();
        let ctx = EscalationContext {
            winner_confidence: 0.40,
            high_risk_blocks: 3,
            unverified_high_risk: 2,
            tournament_tie: false,
            ast_valid: true,
        };
        let d = policy.evaluate(&ctx);
        assert!(d.escalated);
        assert_eq!(
            d.reasons,
            vec![
                EscalationReason::LowWinnerConfidence,
                EscalationReason::HighRiskUnverified
            ]
        );
    }

    #[test]
    fn zero_budget_disables_escalation_entirely() {
        let policy = EscalationPolicy::new(0.99, 0);
        let mut ctx = healthy();
        ctx.winner_confidence = 0.1;
        ctx.tournament_tie = true;
        assert!(!policy.evaluate(&ctx).escalated);
    }

    #[test]
    fn confidence_floor_boundary_is_exclusive() {
        let policy = EscalationPolicy::new(0.65, 3);
        let mut at_floor = healthy();
        at_floor.winner_confidence = 0.65;
        assert!(!policy.evaluate(&at_floor).escalated);
        let mut below = healthy();
        below.winner_confidence = 0.649;
        assert!(policy.evaluate(&below).escalated);
    }
}
