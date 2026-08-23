//! Logprob Extractor & Fine-Grained Reward module.
//! Implements probabilistic confidence, entropy calculation, and token-level verification rewards.

use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerificationVerdict {
    Approved,
    Rejected(String),
    Uncertain(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenLogprob {
    pub token: String,
    pub logprob: f64,
    pub top_logprobs: Vec<(String, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub verdict: VerificationVerdict,
    pub confidence: f64,
    pub entropy: f64,
    pub fine_grained_reward: f64,
}

pub struct LogprobExtractor;

impl LogprobExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Extracts verification metrics and fine-grained rewards from token logprob distribution
    pub fn analyze(&self, text_response: &str, logprobs: &[TokenLogprob]) -> VerificationResult {
        let entropy = self.calculate_entropy(logprobs);
        let confidence = self.calculate_confidence(logprobs);
        let fine_grained_reward = self.calculate_reward(logprobs);

        let verdict = if text_response.contains("APPROVED") && confidence > 0.80 && entropy < 0.85 {
            VerificationVerdict::Approved
        } else if text_response.contains("REJECTED") || confidence < 0.40 {
            VerificationVerdict::Rejected(text_response.to_string())
        } else {
            VerificationVerdict::Uncertain("Ambiguous logprob density or borderline confidence".to_string())
        };

        VerificationResult {
            verdict,
            confidence,
            entropy,
            fine_grained_reward,
        }
    }

    pub fn calculate_entropy(&self, logprobs: &[TokenLogprob]) -> f64 {
        if logprobs.is_empty() {
            return 0.0;
        }

        let total_entropy: f64 = logprobs
            .iter()
            .map(|lp| {
                let probs: Vec<f64> = lp.top_logprobs.iter().map(|(_, lp_val)| lp_val.exp()).collect();
                probs
                    .into_iter()
                    .filter(|&p| p > 1e-9)
                    .map(|p| -p * p.ln())
                    .sum::<f64>()
            })
            .sum();

        total_entropy / logprobs.len() as f64
    }

    pub fn calculate_confidence(&self, logprobs: &[TokenLogprob]) -> f64 {
        if logprobs.is_empty() {
            return 0.85; // Default neutral confidence
        }

        let total_prob: f64 = logprobs
            .iter()
            .map(|lp| {
                let max_logp = lp
                    .top_logprobs
                    .iter()
                    .map(|(_, val)| OrderedFloat(*val))
                    .max()
                    .map(|of| of.0)
                    .unwrap_or(lp.logprob);
                max_logp.exp().clamp(0.0, 1.0)
            })
            .sum();

        total_prob / logprobs.len() as f64
    }

    pub fn calculate_reward(&self, logprobs: &[TokenLogprob]) -> f64 {
        if logprobs.is_empty() {
            return 0.75;
        }

        let score_anchors = [
            ("approved", 1.0),
            ("correct", 0.95),
            ("valid", 0.90),
            ("acceptable", 0.70),
            ("minor", 0.50),
            ("flaw", 0.30),
            ("bug", 0.20),
            ("rejected", 0.0),
        ];

        let mut total_reward = 0.0;
        let mut total_weight = 0.0;

        for lp in logprobs {
            let token_lower = lp.token.to_lowercase();
            for (anchor, score) in &score_anchors {
                if token_lower.contains(anchor) {
                    let weight = lp.logprob.exp().max(0.1);
                    total_reward += score * weight;
                    total_weight += weight;
                }
            }
        }

        if total_weight > 0.0 {
            total_reward / total_weight
        } else {
            0.75
        }
    }
}
