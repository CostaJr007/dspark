//! List-price cost model for the dual-engine thesis (USD / 1M tokens).

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

#[derive(Debug, Clone, Copy)]
pub struct ModelPrice {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
}

/// Cache-miss list prices, USD per 1M tokens (as of 2026-08).
pub fn price_for(model: &str) -> ModelPrice {
    let m = model.to_lowercase();
    if m.contains("gpt-4o-mini") {
        ModelPrice {
            input_per_mtok: 0.15,
            output_per_mtok: 0.60,
        }
    } else if m.contains("gpt-4o") || m.contains("gpt-4.1") {
        ModelPrice {
            input_per_mtok: 2.50,
            output_per_mtok: 10.00,
        }
    } else if m.contains("deepseek-v4-flash") {
        ModelPrice {
            input_per_mtok: 0.14,
            output_per_mtok: 0.28,
        }
    } else if m.contains("deepseek-v4-pro") || m.contains("deepseek-chat") {
        ModelPrice {
            input_per_mtok: 0.435,
            output_per_mtok: 0.87,
        }
    } else if m.contains("gemini") && m.contains("flash") {
        ModelPrice {
            input_per_mtok: 0.15,
            output_per_mtok: 0.60,
        }
    } else if m.contains("claude") && m.contains("haiku") {
        ModelPrice {
            input_per_mtok: 0.25,
            output_per_mtok: 1.25,
        }
    } else if m.starts_with("local:") || m.starts_with("ollama:") || m.starts_with("lmstudio:") {
        ModelPrice {
            input_per_mtok: 0.0,
            output_per_mtok: 0.0,
        }
    } else {
        ModelPrice {
            input_per_mtok: 1.0,
            output_per_mtok: 3.0,
        }
    }
}

pub fn usd_for(model: &str, usage: TokenUsage) -> f64 {
    let p = price_for(model);
    (usage.prompt_tokens as f64 / 1_000_000.0) * p.input_per_mtok
        + (usage.completion_tokens as f64 / 1_000_000.0) * p.output_per_mtok
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
    fn mini_costs_less_than_flagship() {
        let usage = TokenUsage {
            prompt_tokens: 1_000_000,
            completion_tokens: 1_000_000,
        };
        assert!(usd_for("gpt-4o-mini", usage) < usd_for("gpt-4o", usage));
    }
}
