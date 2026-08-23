//! Confidence Head module: Local CPU-based confidence and risk estimator.
//! Estimates code complexity and entropy locally to avoid redundant API verification calls.

use crate::utils::ast_resolver::CodeBlock;
use super::speculative_drafter::DraftTrajectory;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,    // Confidence > 0.88
    Medium, // 0.65 < Confidence <= 0.88
    High,   // Confidence <= 0.65
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockConfidence {
    pub block_id: usize,
    pub function_name: String,
    pub confidence_score: f64,
    pub needs_verification: bool,
    pub risk_level: RiskLevel,
}

pub struct ConfidenceHead {
    pub verification_threshold: f64,
}

impl ConfidenceHead {
    pub fn new(verification_threshold: f64) -> Self {
        Self {
            verification_threshold,
        }
    }
}

impl Default for ConfidenceHead {
    fn default() -> Self {
        Self::new(0.85)
    }
}

impl ConfidenceHead {
    /// Estimates confidence scores across all blocks of a trajectory
    pub fn estimate_confidence(&self, trajectory: &DraftTrajectory) -> Vec<BlockConfidence> {
        trajectory
            .code_blocks
            .iter()
            .enumerate()
            .map(|(idx, block)| {
                let entropy = self.calculate_entropy(block);
                let is_complex = self.detect_complex_logic(block);
                let is_mutating = self.detect_state_mutation(block);

                let mut confidence = 1.0
                    - (0.40 * entropy)
                    - (if is_complex { 0.25 } else { 0.0 })
                    - (if is_mutating { 0.20 } else { 0.0 });

                confidence = confidence.clamp(0.0, 1.0);

                let risk_level = if confidence > 0.88 {
                    RiskLevel::Low
                } else if confidence > 0.65 {
                    RiskLevel::Medium
                } else {
                    RiskLevel::High
                };

                let needs_verification = confidence < self.verification_threshold;

                BlockConfidence {
                    block_id: idx,
                    function_name: block.function_name.clone(),
                    confidence_score: confidence,
                    needs_verification,
                    risk_level,
                }
            })
            .collect()
    }

    fn calculate_entropy(&self, block: &CodeBlock) -> f64 {
        let branch_count = block.code.matches("if ").count()
            + block.code.matches("match ").count()
            + block.code.matches("else").count()
            + block.code.matches("case ").count();

        let loop_count = block.code.matches("for ").count()
            + block.code.matches("while ").count()
            + block.code.matches("loop {").count();

        let recursive = if !block.function_name.is_empty() {
            block.code.matches(&block.function_name).count() > 1
        } else {
            false
        };

        let raw_complexity = (branch_count as f64 * 0.25)
            + (loop_count as f64 * 0.35)
            + (if recursive { 0.5 } else { 0.0 });

        (raw_complexity / 5.0).min(1.0)
    }

    fn detect_complex_logic(&self, block: &CodeBlock) -> bool {
        block.code.contains("async ")
            || block.code.contains("unsafe ")
            || block.code.contains("trait ")
            || block.code.contains("impl ")
            || block.code.contains("*mut ")
            || block.code.contains("*const ")
            || block.code.contains("malloc(")
    }

    fn detect_state_mutation(&self, block: &CodeBlock) -> bool {
        block.code.contains("mut ")
            || block.code.contains("static ")
            || block.code.contains("global ")
            || block.code.contains("RefCell")
            || block.code.contains("Mutex")
    }
}
