//! Speculative Engine orchestration modules.

pub mod confidence_head;
pub mod cost_scheduler;
pub mod escalation;
pub mod logprob_extractor;
pub mod pivot_tournament;
pub mod speculative_drafter;

pub use confidence_head::{BlockConfidence, ConfidenceHead, RiskLevel};
pub use cost_scheduler::{CostScheduler, VerificationPlan};
pub use escalation::{EscalationContext, EscalationDecision, EscalationPolicy, EscalationReason};
pub use logprob_extractor::{LogprobExtractor, TokenLogprob, VerificationResult, VerificationVerdict};
pub use pivot_tournament::{tournament_comparison_count, PivotTournament, TournamentResult};
pub use speculative_drafter::{DraftTrajectory, SpeculativeDrafter};
