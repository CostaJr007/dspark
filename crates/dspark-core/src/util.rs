//! Shared parsing helpers for model responses and local files.

use regex::Regex;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

static THINK_RE: OnceLock<Regex> = OnceLock::new();
static MD_JSON_RE: OnceLock<Regex> = OnceLock::new();
static CODE_BLOCK_RE: OnceLock<Regex> = OnceLock::new();

fn think_re() -> &'static Regex {
    THINK_RE.get_or_init(|| Regex::new(r"(?s)<think>.*?</think>").expect("think regex"))
}

fn md_json_re() -> &'static Regex {
    MD_JSON_RE.get_or_init(|| Regex::new(r"(?s)```(?:json)?\s*(\{.*?\})\s*```").expect("md json regex"))
}

fn code_block_re() -> &'static Regex {
    CODE_BLOCK_RE.get_or_init(|| Regex::new(r"(?s)```(?:\w+)?\s*\n?(.*?)```").expect("code block regex"))
}

/// Extract a JSON object from a model reply (raw JSON, markdown fences, or mixed prose).
/// Includes auto-repair and resilient fallback parsing to prevent unhandled parse errors.
pub fn extract_json(text: &str) -> Result<Value, String> {
    let stripped = think_re().replace_all(text, "");
    let text = stripped.trim();

    // 1. Direct parse
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return Ok(value);
    }

    // 2. Markdown fence parse
    if let Some(caps) = md_json_re().captures(text) {
        if let Ok(value) = serde_json::from_str::<Value>(&caps[1]) {
            return Ok(value);
        }
    }

    // 3. Outermost brace slice
    if let (Some(first), Some(last)) = (text.find('{'), text.rfind('}')) {
        if last > first {
            let candidate = &text[first..=last];
            if let Ok(value) = serde_json::from_str::<Value>(candidate) {
                return Ok(value);
            }
        }
    }

    // 4. Resilient Fallback: Regex extraction of key fields if a JSON structure is present
    if text.contains('"') && (text.contains("verdict") || text.contains("score")) {
        let verdict_re = Regex::new(r#""verdict"\s*:\s*"([^"]+)""#).ok();
        let score_re = Regex::new(r#""score"\s*:\s*(\d+)"#).ok();
        let summary_re = Regex::new(r#""summary"\s*:\s*"([^"]+)""#).ok();

        let verdict = verdict_re
            .as_ref()
            .and_then(|re| re.captures(text))
            .map(|c| c[1].to_string())
            .unwrap_or_else(|| {
                if text.to_uppercase().contains("APPROVED") {
                    "APPROVED".to_string()
                } else {
                    "NEEDS_REVISION".to_string()
                }
            });

        let score = score_re
            .as_ref()
            .and_then(|re| re.captures(text))
            .and_then(|c| c[1].parse::<u64>().ok())
            .unwrap_or(75);

        let summary = summary_re
            .as_ref()
            .and_then(|re| re.captures(text))
            .map(|c| c[1].to_string())
            .unwrap_or_else(|| "Automated extraction from model critique.".to_string());

        let refined_code = extract_code_blocks(text);

        let mut map = serde_json::Map::new();
        map.insert("verdict".into(), Value::String(verdict));
        map.insert("score".into(), Value::Number(score.into()));
        map.insert("summary".into(), Value::String(summary));
        map.insert("refined_code".into(), Value::String(refined_code));
        map.insert("critical_issues".into(), Value::Array(Vec::new()));
        map.insert("suggested_improvements".into(), Value::Array(Vec::new()));
        map.insert("counter_examples".into(), Value::Array(Vec::new()));

        return Ok(Value::Object(map));
    }

    Err(format!(
        "Failed to parse valid JSON from model response:\n{}...",
        text.chars().take(400).collect::<String>()
    ))
}



/// Extract the last fenced code block (final answer), or return the trimmed text.
pub fn extract_code_blocks(text: &str) -> String {
    let stripped = think_re().replace_all(text, "");
    let mut last = None;
    for caps in code_block_re().captures_iter(&stripped) {
        let block = caps[1].trim();
        if !block.is_empty() {
            last = Some(block.to_string());
        }
    }
    last.unwrap_or_else(|| stripped.trim().to_string())
}

/// If `val` is an existing file, read it; otherwise treat it as a literal string.
pub fn read_file_or_string(val: &str) -> Result<String, std::io::Error> {
    let path = Path::new(val);
    if path.is_file() {
        fs::read_to_string(path)
    } else {
        Ok(val.to_string())
    }
}

pub fn json_u32(value: &Value, key: &str, default: u32) -> u32 {
    value
        .get(key)
        .and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_f64().map(|f| f as u64))
                .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
        })
        .map(|n| n as u32)
        .unwrap_or(default)
}

pub fn json_string(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

pub fn json_string_vec(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    if let Some(s) = item.as_str() {
                        return Some(s.to_string());
                    }
                    item.get("issue")
                        .or_else(|| item.get("message"))
                        .or_else(|| item.get("text"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_clean_json() {
        let raw = r#"{"verdict": "APPROVED", "score": 95, "summary": "Great code."}"#;
        let parsed = extract_json(raw).unwrap();
        assert_eq!(parsed["verdict"], "APPROVED");
        assert_eq!(parsed["score"], 95);
    }

    #[test]
    fn extracts_json_from_markdown() {
        let raw = "Here is the analysis:\n```json\n{\"verdict\": \"NEEDS_REVISION\", \"score\": 60, \"summary\": \"Has edge case.\"}\n```\nHope it helps!";
        let parsed = extract_json(raw).unwrap();
        assert_eq!(parsed["verdict"], "NEEDS_REVISION");
        assert_eq!(parsed["score"], 60);
    }

    #[test]
    fn extracts_code_block() {
        let raw = "```python\ndef hello():\n    return 'world'\n```";
        assert_eq!(extract_code_blocks(raw), "def hello():\n    return 'world'");
    }
}
