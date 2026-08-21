//! DeepSeek Curator: audit, refine, and arbitrate implementations.

use crate::client::{ClientError, ModelClient};
use crate::prompts::{ARBITRATOR_SYSTEM_PROMPT, REFINER_SYSTEM_PROMPT};
use crate::verifier::VERIFIER_SYSTEM_PROMPT;
use crate::util::{extract_code_blocks, extract_json, json_string, json_string_vec, json_u32};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CuratorError {
    #[error("Client error: {0}")]
    Client(#[from] ClientError),
    #[error("{0}")]
    Parse(String),
    #[error("{0}")]
    Invalid(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CurationVerdict {
    Approved,
    NeedsRevision,
    Rejected,
}

impl CurationVerdict {
    pub fn from_str_loose(raw: &str) -> Self {
        match raw.to_uppercase().as_str() {
            "APPROVED" => Self::Approved,
            "REJECTED" => Self::Rejected,
            _ => Self::NeedsRevision,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Approved => "APPROVED",
            Self::NeedsRevision => "NEEDS_REVISION",
            Self::Rejected => "REJECTED",
        }
    }
}

impl std::fmt::Display for CurationVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EdgeCase {
    pub case: String,
    pub risk_level: String,
    pub handled_properly: bool,
    #[serde(default)]
    pub remedy: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CounterExample {
    pub failing_input: String,
    pub expected_behavior: String,
    pub actual_behavior: String,
    #[serde(default = "default_severity")]
    pub severity: String,
}

fn default_severity() -> String {
    "HIGH".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
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
    pub criteria_scores: Value,
    #[serde(default)]
    pub counter_examples: Vec<CounterExample>,
    #[serde(default)]
    pub io_contract_analysis: Value,
    #[serde(default)]
    pub edge_cases: Vec<EdgeCase>,
    #[serde(default)]
    pub complexity: Option<ComplexityAnalysis>,
    #[serde(default)]
    pub critical_issues: Vec<String>,
    #[serde(default)]
    pub suggested_improvements: Vec<String>,
    #[serde(default)]
    pub refined_code: Option<String>,
    #[serde(default)]
    pub raw_response: String,
}

impl AuditResult {
    pub fn is_approved(&self) -> bool {
        self.verdict == CurationVerdict::Approved
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RefineResult {
    pub refined_code: String,
    #[serde(default)]
    pub summary_of_changes: Vec<String>,
    #[serde(default)]
    pub raw_response: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArbitrationResult {
    pub winner_index: i64,
    pub rationale: String,
    #[serde(default)]
    pub comparison_matrix: Value,
    #[serde(default)]
    pub synthesized_code: String,
    #[serde(default)]
    pub raw_response: String,
}

pub struct DeepSeekCurator {
    client: ModelClient,
}

impl DeepSeekCurator {
    pub fn new() -> Result<Self, CuratorError> {
        Ok(Self {
            client: ModelClient::from_spec(
                &std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-v4-flash".into()),
            )?,
        })
    }

    pub fn with_model(spec: &str) -> Result<Self, CuratorError> {
        Ok(Self {
            client: ModelClient::from_spec(spec)?,
        })
    }

    pub fn with_client(client: ModelClient) -> Self {
        Self { client }
    }

    async fn complete_json(
        &self,
        prompt: &str,
        system: &str,
        temperature: f32,
    ) -> Result<String, CuratorError> {
        match self
            .client
            .complete(prompt, Some(system), temperature, true)
            .await
        {
            Ok(s) => Ok(s),
            Err(_) => Ok(self
                .client
                .complete(prompt, Some(system), temperature, false)
                .await?),
        }
    }

    pub async fn audit(
        &self,
        code: &str,
        specification: &str,
        language: Option<&str>,
    ) -> Result<AuditResult, CuratorError> {
        let lang_str = language
            .map(|l| format!("Language: {}\n", l))
            .unwrap_or_default();
        let lang_fence = language.unwrap_or("");
        let user_prompt = format!(
            "### SPECIFICATION / REQUIREMENTS:\n{specification}\n\n\
             ### CANDIDATE IMPLEMENTATION TO AUDIT:\n{lang_str}```{lang_fence}\n{code}\n```\n\n\
             Perform strict reasoning audit, I/O verification, edge case simulation, and return the required JSON."
        );

        let raw_resp = self
            .complete_json(&user_prompt, VERIFIER_SYSTEM_PROMPT, 0.1)
            .await?;
        let data = extract_json(&raw_resp).map_err(CuratorError::Parse)?;
        Ok(audit_from_value(&data, raw_resp))
    }

    pub async fn refine(
        &self,
        code: &str,
        specification: &str,
        feedback: Option<&str>,
        language: Option<&str>,
    ) -> Result<RefineResult, CuratorError> {
        let feedback_section = feedback
            .map(|f| format!("### AUDIT FEEDBACK / ISSUES TO FIX:\n{f}\n\n"))
            .unwrap_or_default();
        let lang_str = language
            .map(|l| format!("Language: {}\n", l))
            .unwrap_or_default();
        let lang_fence = language.unwrap_or("");
        let user_prompt = format!(
            "### SPECIFICATION:\n{specification}\n\n{feedback_section}\
             ### DRAFT CODE:\n{lang_str}```{lang_fence}\n{code}\n```\n\n\
             Refine the code to 100% production readiness. Fix all potential edge cases and enforce strict I/O typing."
        );

        let raw_resp = self
            .client
            .complete(&user_prompt, Some(REFINER_SYSTEM_PROMPT), 0.2, false)
            .await?;
        let refined_code = extract_code_blocks(&raw_resp);

        let changes: Vec<String> = raw_resp
            .lines()
            .map(str::trim)
            .filter(|line| {
                (line.starts_with("- ") || line.starts_with("* ") || line.starts_with("• "))
                    && !line.starts_with("```")
            })
            .map(|line| line.chars().skip(2).collect::<String>().trim().to_string())
            .collect();

        Ok(RefineResult {
            refined_code,
            summary_of_changes: changes,
            raw_response: raw_resp,
        })
    }

    pub async fn arbitrate(
        &self,
        candidates: &[String],
        specification: &str,
        language: Option<&str>,
    ) -> Result<ArbitrationResult, CuratorError> {
        if candidates.len() < 2 {
            return Err(CuratorError::Invalid(
                "Arbitration requires at least 2 candidate implementations.".into(),
            ));
        }

        let lang_fence = language.unwrap_or("");
        let mut body = format!("### SPECIFICATION:\n{specification}\n\n");
        for (idx, cand) in candidates.iter().enumerate() {
            body.push_str(&format!(
                "### CANDIDATE #{idx}:\n```{lang_fence}\n{cand}\n```\n\n"
            ));
        }
        body.push_str(
            "Compare the candidates thoroughly, choose the winner and synthesize the optimal code in the specified JSON format.",
        );

        let raw_resp = self
            .complete_json(&body, ARBITRATOR_SYSTEM_PROMPT, 0.1)
            .await?;
        let data = extract_json(&raw_resp).map_err(CuratorError::Parse)?;

        let winner_index = data
            .get("winner_index")
            .and_then(|v| v.as_i64().or_else(|| v.as_u64().map(|n| n as i64)))
            .unwrap_or(0);

        Ok(ArbitrationResult {
            winner_index,
            rationale: json_string(&data, "rationale"),
            comparison_matrix: data
                .get("comparison_matrix")
                .cloned()
                .unwrap_or(Value::Object(Default::default())),
            synthesized_code: json_string(&data, "synthesized_code"),
            raw_response: raw_resp,
        })
    }
}

fn audit_from_value(data: &Value, raw: String) -> AuditResult {
    let verdict = CurationVerdict::from_str_loose(&json_string(data, "verdict"));

    let mut edge_cases = Vec::new();
    let edge_src = data
        .get("edge_cases_identified")
        .or_else(|| data.get("edge_cases"))
        .and_then(|v| v.as_array());
    if let Some(arr) = edge_src {
        for ec in arr {
            edge_cases.push(EdgeCase {
                case: json_string(ec, "case"),
                risk_level: ec
                    .get("risk_level")
                    .and_then(|v| v.as_str())
                    .unwrap_or("MEDIUM")
                    .to_string(),
                handled_properly: ec
                    .get("handled_properly")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                remedy: json_string(ec, "remedy"),
            });
        }
    }

    let mut counter_examples = Vec::new();
    if let Some(arr) = data.get("counter_examples").and_then(|v| v.as_array()) {
        for ce in arr {
            counter_examples.push(CounterExample {
                failing_input: json_string(ce, "failing_input"),
                expected_behavior: json_string(ce, "expected_behavior"),
                actual_behavior: json_string(ce, "actual_behavior"),
                severity: ce
                    .get("severity")
                    .and_then(|v| v.as_str())
                    .unwrap_or("HIGH")
                    .to_string(),
            });
        }
    }

    let complexity = data.get("complexity").map(|c| ComplexityAnalysis {
        time: json_string(c, "time"),
        space: json_string(c, "space"),
        optimal: c.get("optimal").and_then(|v| v.as_bool()).unwrap_or(false),
    });

    let refined = json_string(data, "refined_code");
    let refined_code = if refined.trim().is_empty() {
        None
    } else {
        Some(refined)
    };

    AuditResult {
        verdict,
        score: json_u32(data, "score", 70),
        summary: json_string(data, "summary"),
        criteria_scores: data
            .get("criteria_scores")
            .cloned()
            .unwrap_or(Value::Object(Default::default())),
        counter_examples,
        io_contract_analysis: data
            .get("io_contract_analysis")
            .cloned()
            .unwrap_or(Value::Object(Default::default())),
        edge_cases,
        complexity,
        critical_issues: json_string_vec(data, "critical_issues"),
        suggested_improvements: json_string_vec(data, "suggested_improvements"),
        refined_code,
        raw_response: raw,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approved_verdict() {
        let data = serde_json::json!({
            "verdict": "APPROVED",
            "score": 95,
            "summary": "All tests passed",
            "edge_cases_identified": [{
                "case": "Empty array",
                "risk_level": "LOW",
                "handled_properly": true
            }]
        });
        let res = audit_from_value(&data, String::new());
        assert!(res.is_approved());
        assert_eq!(res.score, 95);
        assert_eq!(res.edge_cases.len(), 1);
    }
}
