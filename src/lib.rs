//! DSpark: Dual-LLM speculative engine and autonomous agent, implemented in Rust.

pub mod agent;
pub mod benchmark;
pub mod client;
pub mod curator;
pub mod generator;
pub mod grok_agent;
pub mod mcp;
pub mod pair;
pub mod pipeline;
pub mod prompts;
pub mod repl;
pub mod search;
pub mod tools;
pub mod util;
pub mod verifier;

pub use agent::DSparkAgent;
pub use client::{DeepSeekClient, GeminiClient, LocalLLMClient, ModelClient, OpenAIClient};
pub use curator::DeepSeekCurator;
pub use generator::GeminiGenerator;
pub use pair::DsparkPair;
pub use pipeline::DSparkPipeline;
pub use search::WebSearchEngine;
