//! Speculative Engine orchestration modules.

pub mod confidence_head;
pub mod cost_scheduler;
pub mod logprob_extractor;
pub mod pivot_tournament;
pub mod speculative_drafter;

pub use confidence_head::{BlockConfidence, ConfidenceHead, RiskLevel};
pub use cost_scheduler::{CostScheduler, VerificationPlan};
pub use logprob_extractor::{LogprobExtractor, TokenLogprob, VerificationResult, VerificationVerdict};
pub use pivot_tournament::{PivotTournament, TournamentResult};
pub use speculative_drafter::{DraftTrajectory, SpeculativeDrafter};
