//! DeepSeek Curator & I/O Verification Engine in Rust.

use crate::client::{ClientError, DeepSeekClient};
use crate::prompts::CURATOR_SYSTEM_PROMPT;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CuratorError {
    #[error("Client error: {0}")]
    Client(#[from] ClientError),
    #[error("Failed to parse verification JSON: {0}")]
    Parse(#[from] serde_json::Error),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CurationVerdict {
    APPROVED,
    NEEDS_REVISION,
    REJECTED,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EdgeCase {
    pub case: String,
    pub risk_level: String,
    pub handled_properly: bool,
    #[serde(default)]
    pub remedy: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplexityAnalysis {
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub space: String,
    #[serde(default)]
    pub optimal: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditResult {
    pub verdict: CurationVerdict,
    pub score: u32,
    pub summary: String,
    #[serde(default)]
    pub edge_cases_identified: Vec<EdgeCase>,
    #[serde(default)]
    pub complexity: Option<ComplexityAnalysis>,
    #[serde(default)]
    pub critical_issues: Vec<String>,
    #[serde(default)]
    pub suggested_improvements: Vec<String>,
    #[serde(default)]
    pub refined_code: Option<String>,
}

pub struct DeepSeekCurator {
    client: DeepSeekClient,
}

impl DeepSeekCurator {
    pub fn new() -> Result<Self, CuratorError> {
        Ok(Self {
            client: DeepSeekClient::new()?,
        })
    }

    pub async fn audit(&self, code: &str, specification: &str) -> Result<AuditResult, CuratorError> {
        let user_prompt = format!(
            "### SPECIFICATION / REQUIREMENTS:\n{}\n\n### CANDIDATE IMPLEMENTATION:\n```\n{}\n```\n\nPerform strict reasoning audit and return the requested JSON schema.",
            specification, code
        );

        let raw_resp = self
            .client
            .complete(&user_prompt, Some(CURATOR_SYSTEM_PROMPT), 0.1, true)
            .await?;

        let result: AuditResult = serde_json::from_str(&raw_resp)?;
        Ok(result)
    }
}
