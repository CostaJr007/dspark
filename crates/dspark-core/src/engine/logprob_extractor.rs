//! Logprob Extractor & Fine-Grained Reward module.
//! Implements probabilistic confidence, entropy calculation, and token-level verification rewards.
//!
//! Continuous rewards (LLM-as-a-Verifier, Kwok et al., arXiv:2607.05391, Eq. 3.1):
//! `R(x, tau) = (1/CK) * sum_c sum_k sum_g p_theta(v_g | x, c, tau) * phi(v_g)` --
//! the expectation over the distribution of scoring-token logits, instead of the
//! discrete argmax score a standard LM judge collapses to. Also provides the
//! two-stage workaround (Appendix B.6) for logit-restricted frontier models.

use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

/// 1-20 letter-based score vocabulary (the paper uses letters so that the logprob
/// extraction happens over distinct single tokens).
pub const SCORE_LETTERS: &str = "ABCDEFGHIJKLMNOPQRST";

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
}

impl Default for LogprobExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl LogprobExtractor {
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

    /// Maps a score token to its scalar value phi(v) in [0, 1].
    ///
    /// Accepts either the letter-based 1-G vocabulary (A..T for G = 20) or integer
    /// strings "1".."G"; returns None for tokens outside the score vocabulary.
    pub fn score_token_value(token: &str, granularity: usize) -> Option<f64> {
        let t = token.trim();
        if t.is_empty() || granularity < 2 {
            return None;
        }
        if let Ok(v) = t.parse::<usize>() {
            if (1..=granularity).contains(&v) {
                return Some((v - 1) as f64 / (granularity - 1) as f64);
            }
            return None;
        }
        let letter = t.chars().next()?.to_ascii_uppercase();
        // Single-letter tokens only: "banana" is not a score token.
        if t.chars().count() == 1 && letter.is_ascii_uppercase() {
            let idx = letter as usize - 'A' as usize;
            if idx < granularity {
                return Some(idx as f64 / (granularity - 1) as f64);
            }
        }
        None
    }

    /// Continuous fine-grained reward (Eq. 3.1): the expectation of the scoring-token
    /// distribution, `R = sum_g p(v_g) * phi(v_g)`.
    ///
    /// `score_logprobs` are (token, logprob) pairs read at the `<score>` tag position
    /// of the verifier prompt. The expectation is renormalized over the mass placed
    /// on recognized score tokens so that off-vocabulary mass does not bias it.
    pub fn continuous_reward(&self, score_logprobs: &[(String, f64)], granularity: usize) -> f64 {
        if score_logprobs.is_empty() {
            return 0.5;
        }
        let mut weighted = 0.0;
        let mut mass = 0.0;
        for (token, logprob) in score_logprobs {
            if let Some(phi) = Self::score_token_value(token, granularity) {
                let p = logprob.exp();
                weighted += p * phi;
                mass += p;
            }
        }
        if mass > 1e-9 {
            weighted / mass
        } else {
            0.5
        }
    }

    /// Two-stage workaround for logit-restricted frontier models (Appendix B.6):
    /// stage 1 asks the closed model for a free-form `<reasoning>` block, stage 2
    /// routes that reasoning to a logprob-accessible verifier and reads its
    /// scoring-token logprobs, which carry the calibrated distribution the closed
    /// API withholds. This computes the stage-2 continuous reward; `closed_reasoning`
    /// is already folded into `open_score_logprobs` by construction.
    pub fn two_stage_continuous_reward(
        &self,
        _closed_reasoning: &str,
        open_score_logprobs: &[(String, f64)],
        granularity: usize,
    ) -> f64 {
        self.continuous_reward(open_score_logprobs, granularity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_token_value_maps_letters_and_integers() {
        assert_eq!(LogprobExtractor::score_token_value("A", 20), Some(0.0));
        assert_eq!(LogprobExtractor::score_token_value("T", 20), Some(1.0));
        assert_eq!(LogprobExtractor::score_token_value("1", 20), Some(0.0));
        assert_eq!(LogprobExtractor::score_token_value("20", 20), Some(1.0));
        assert!(LogprobExtractor::score_token_value("U", 20).is_none());
        assert!(LogprobExtractor::score_token_value("21", 20).is_none());
        assert!(LogprobExtractor::score_token_value("banana", 20).is_none());
    }

    #[test]
    fn continuous_reward_is_the_logprob_expectation() {
        let extractor = LogprobExtractor::new();

        // Mass on "T" (value 1.0) and "A" (value 0.0): expectation = p_T / (p_T + p_A).
        let logprobs = vec![
            ("A".to_string(), 0.0f64), // logprob 0 -> p = 1.0
            ("T".to_string(), -1.0f64), // p = e^-1
        ];
        let p_t = (-1.0f64).exp();
        let expected = p_t / (1.0 + p_t);
        let reward = extractor.continuous_reward(&logprobs, 20);
        assert!((reward - expected).abs() < 1e-6, "{reward} vs {expected}");

        // Uniform mass over the full 20-letter vocabulary -> 0.5.
        let uniform: Vec<(String, f64)> = (0..20)
            .map(|i| {
                let letter = (b'A' + i as u8) as char;
                (letter.to_string(), 0.0f64)
            })
            .collect();
        let reward = extractor.continuous_reward(&uniform, 20);
        assert!((reward - 0.5).abs() < 1e-6, "{reward}");
    }

    #[test]
    fn two_stage_reward_delegates_to_continuous_expectation() {
        let extractor = LogprobExtractor::new();
        let open_logprobs = vec![("T".to_string(), 0.0f64)];
        let r = extractor.two_stage_continuous_reward(
            "Trajectory A correctly validates equivalence on the database.",
            &open_logprobs,
            20,
        );
        assert!((r - 1.0).abs() < 1e-6);
    }
}
