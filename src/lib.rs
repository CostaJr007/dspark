//! DSpark: Dual-LLM Speculative Engine & Autonomous Agent in Rust.

pub mod agent;
pub mod client;
pub mod curator;
pub mod prompts;
pub mod repl;
pub mod search;

pub use agent::DSparkAgent;
pub use client::DeepSeekClient;
pub use curator::DeepSeekCurator;
pub use search::WebSearchEngine;
