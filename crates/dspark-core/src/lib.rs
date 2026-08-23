//! DSpark: Speculative Orchestration and Dual-Engine verification CLI & library.

pub mod agent;
pub mod agent_loop;
pub mod client;
pub mod cost;
pub mod curator;
pub mod engine;
pub mod generator;
pub mod mcp;
pub mod oracle;
pub mod pair;
pub mod pipeline;
pub mod prompts;
pub mod repl;
pub mod search;
pub mod tools;
pub mod util;
pub mod utils;
pub mod verifier;

pub use agent::DSparkAgent;
pub use client::{DeepSeekClient, GeminiClient, LocalLLMClient, ModelClient, OpenAIClient};
pub use cost::TokenUsage;
pub use curator::DeepSeekCurator;
pub use engine::{
    BlockConfidence, ConfidenceHead, CostScheduler, DraftTrajectory, LogprobExtractor,
    PivotTournament, RiskLevel, SpeculativeDrafter, TokenLogprob, TournamentResult,
    VerificationPlan, VerificationResult, VerificationVerdict,
};
pub use generator::GeminiGenerator;
pub use pair::DsparkPair;
pub use pipeline::DSparkPipeline;
pub use search::WebSearchEngine;
pub use utils::{AstResolver, CodeBlock, DependencyGraph, PromptOptimizer};
