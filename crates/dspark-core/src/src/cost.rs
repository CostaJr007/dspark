//! Token usage counters from provider responses.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

impl TokenUsage {
    pub fn add(&mut self, other: TokenUsage) {
        self.prompt_tokens += other.prompt_tokens;
        self.completion_tokens += other.completion_tokens;
    }

    pub fn total(self) -> u64 {
        self.prompt_tokens + self.completion_tokens
    }
}

pub fn extract_usage(value: &serde_json::Value) -> TokenUsage {
    let u = value.get("usage");
    TokenUsage {
        prompt_tokens: u
            .and_then(|v| v.get("prompt_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        completion_tokens: u
            .and_then(|v| v.get("completion_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_usage_tokens() {
        let v = serde_json::json!({"usage": {"prompt_tokens": 10, "completion_tokens": 20}});
        let u = extract_usage(&v);
        assert_eq!(u.prompt_tokens, 10);
        assert_eq!(u.completion_tokens, 20);
        assert_eq!(u.total(), 30);
    }
}
