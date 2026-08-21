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
    CODE_BLOCK_RE.get_or_init(|| Regex::new(r"(?s)```(?:\w+)?\n(.*?)```").expect("code block regex"))
}

/// Extract a JSON object from a model reply (raw JSON, markdown fences, or mixed prose).
pub fn extract_json(text: &str) -> Result<Value, String> {
    let stripped = think_re().replace_all(text, "");
    let text = stripped.trim();

    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return Ok(value);
    }

    if let Some(caps) = md_json_re().captures(text) {
        if let Ok(value) = serde_json::from_str::<Value>(&caps[1]) {
            return Ok(value);
        }
    }

    if let (Some(first), Some(last)) = (text.find('{'), text.rfind('}')) {
        if last > first {
            if let Ok(value) = serde_json::from_str::<Value>(&text[first..=last]) {
                return Ok(value);
            }
        }
    }

    Err(format!(
        "Failed to parse valid JSON from model response:\n{}...",
        text.chars().take(400).collect::<String>()
    ))
}

/// Extract the first fenced code block, or return the trimmed text.
pub fn extract_code_blocks(text: &str) -> String {
    if let Some(caps) = code_block_re().captures(text) {
        return caps[1].trim().to_string();
    }
    text.trim().to_string()
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
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

pub fn json_string_vec(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
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
