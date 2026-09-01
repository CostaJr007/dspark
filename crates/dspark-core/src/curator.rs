//! DeepSeek Curator: audit, refine, and arbitrate implementations.

use crate::client::{ClientError, ModelClient};
use crate::oracle::{run_python_spec_oracle, OracleFailure};
use crate::prompts::{ARBITRATOR_SYSTEM_PROMPT, REFINER_SYSTEM_PROMPT};
use crate::util::{extract_code_blocks, extract_json, json_string, json_string_vec, json_u32};
use crate::verifier::VERIFIER_SYSTEM_PROMPT;
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

    /// True when the draft has genuine critical failures or severe contract violations.
    pub fn must_revise(&self) -> bool {
        self.verdict != CurationVerdict::Approved
            || self.counter_examples.iter().any(|c| c.severity == "CRITICAL" || c.severity == "HIGH")
            || !self.critical_issues.is_empty()
            || self.score < 80
    }

    fn reconcile(&mut self) {
        let has_severe_failures = self
            .counter_examples
            .iter()
            .any(|c| c.severity == "CRITICAL" || c.severity == "HIGH")
            || !self.critical_issues.is_empty();

        if has_severe_failures {
            if self.verdict == CurationVerdict::Approved {
                self.verdict = CurationVerdict::NeedsRevision;
            }
            if self.score > 70 {
                self.score = 70;
            }
        }
    }


    fn apply_oracle(&mut self, failures: Vec<OracleFailure>) {
        if failures.is_empty() {
            return;
        }
        for f in failures {
            let msg = if f.message.is_empty() {
                format!("{}: {} vs {}", f.kind, f.expected, f.actual)
            } else {
                f.message.clone()
            };
            self.critical_issues.push(format!("spec-oracle: {msg}"));
            self.counter_examples.push(CounterExample {
                failing_input: f.input,
                expected_behavior: f.expected,
                actual_behavior: if f.actual.is_empty() { msg } else { f.actual },
                severity: "HIGH".into(),
            });
        }
        self.verdict = CurationVerdict::NeedsRevision;
        if self.score > 50 {
            self.score = 50;
        }
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
        let spec = std::env::var("DSPARK_CURATOR")
            .ok()
            .or_else(|| std::env::var("DEEPSEEK_MODEL").ok())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| crate::pair::DsparkPair::load().curator);
        Ok(Self {
            client: ModelClient::from_spec(&spec)?,
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

    pub fn usage(&self) -> crate::cost::TokenUsage {
        self.client.usage()
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
            Ok(s) if !s.trim().is_empty() => Ok(s),
            Ok(_) | Err(_) => self
                .client
                .complete(prompt, Some(system), temperature, false)
                .await
                .map_err(CuratorError::from),
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
             Execute every example in the specification (doctest / >>> / encode-decode roundtrip). \
             APPROVED is forbidden if any of those fail. Return the required JSON."
        );

        let raw_resp = self
            .complete_json(&user_prompt, VERIFIER_SYSTEM_PROMPT, 0.1)
            .await?;
        let data = extract_json(&raw_resp).map_err(CuratorError::Parse)?;
        let mut audit = audit_from_value(&data, raw_resp);
        audit.reconcile();
        let py = language
            .map(|l| l.eq_ignore_ascii_case("python") || l.eq_ignore_ascii_case("py"))
            .unwrap_or_else(|| looks_like_python(code, specification));
        if py {
            audit.apply_oracle(run_python_spec_oracle(code, specification));
        }
        Ok(audit)
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
             Refine the code so every specification example passes. Return only a markdown code block."
        );

        let raw_resp = self
            .client
            .complete(&user_prompt, Some(REFINER_SYSTEM_PROMPT), 0.2, false)
            .await?;
        let mut refined_code = extract_refined_source(&raw_resp);
        if refined_code.trim().is_empty() {
            refined_code = code.to_string();
        }

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

        let raw_index = data
            .get("winner_index")
            .and_then(|v| v.as_i64().or_else(|| v.as_u64().map(|n| n as i64)))
            .unwrap_or(0);
        let winner_index = if raw_index >= 0 && (raw_index as usize) < candidates.len() {
            raw_index
        } else {
            0
        };

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

    let mut audit = AuditResult {
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
    };
    audit.reconcile();
    audit
}

fn extract_refined_source(raw: &str) -> String {
    if let Ok(data) = extract_json(raw) {
        let from_json = json_string(&data, "refined_code");
        let from_json = if from_json.trim().is_empty() {
            json_string(&data, "code")
        } else {
            from_json
        };
        if !from_json.trim().is_empty() {
            return extract_code_blocks(&from_json);
        }
    }
    extract_code_blocks(raw)
}

fn looks_like_python(code: &str, spec: &str) -> bool {
    let blob = format!("{code}\n{spec}").to_lowercase();
    blob.contains("def ") || blob.contains(">>> ") || blob.contains("import ")
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
        assert!(!res.must_revise());
        assert_eq!(res.score, 95);
        assert_eq!(res.edge_cases.len(), 1);
    }

    #[test]
    fn approved_with_counter_example_is_forced_to_revise() {
        let data = serde_json::json!({
            "verdict": "APPROVED",
            "score": 100,
            "summary": "Looks fine",
            "counter_examples": [{
                "failing_input": "[]",
                "expected_behavior": "[]",
                "actual_behavior": "None"
            }]
        });
        let res = audit_from_value(&data, String::new());
        assert!(!res.is_approved());
        assert!(res.must_revise());
        assert!(res.score <= 70);
    }

    #[test]
    fn extract_refined_from_json_blob() {
        let raw = r#"{"refined_code": "def f():\n    return 1"}"#;
        assert!(extract_refined_source(raw).contains("def f()"));
    }
}
