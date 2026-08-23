//! Prompt Optimizer module: Prefix-Cache Friendly Prompts.
//! Places large code contexts in prompt prefix and evaluation instructions in the tail to maximize KV cache reuse.

use super::ast_resolver::CodeBlock;

pub struct PromptOptimizer;

impl PromptOptimizer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PromptOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptOptimizer {
    /// Generates cache-friendly trajectory comparison prompt
    pub fn generate_comparison_prompt(
        &self,
        blocks_a: &[CodeBlock],
        blocks_b: &[CodeBlock],
        criteria: &str,
    ) -> String {
        let code_a: String = blocks_a.iter().map(|b| b.code.as_str()).collect::<Vec<_>>().join("\n\n");
        let code_b: String = blocks_b.iter().map(|b| b.code.as_str()).collect::<Vec<_>>().join("\n\n");

        format!(
            r#"# Trajectory Comparison Task

## Candidate A Implementation
```
{}
```

## Candidate B Implementation
```
{}
```

## Comparative Evaluation Criteria
{}

## Verdict Rules
Compare candidate implementations strictly against the criteria above.
Return JSON with the format:
```json
{{
  "winner": "A" | "B" | "EQUAL",
  "rationale": "Concise reasoning for decision"
}}
```
"#,
            code_a,
            code_b,
            criteria
        )
    }

    /// Generates cache-friendly contract verification prompt
    pub fn generate_contract_prompt(
        &self,
        code: &str,
        preconditions: &[String],
        postconditions: &[String],
        invariants: &[String],
    ) -> String {
        format!(
            r#"# Formal Contract Verification Task

## Source Implementation
```
{}
```

## Formal Contract Specification
### Preconditions:
{}

### Postconditions:
{}

### Invariants:
{}

## Instructions:
Audit the source against all preconditions, postconditions, and invariants.
Report:
- APPROVED: If all contracts and boundary edge cases are satisfied.
- REJECTED: If any contract is violated (provide counterexample).
"#,
            code,
            preconditions.iter().map(|p| format!("- {}", p)).collect::<Vec<_>>().join("\n"),
            postconditions.iter().map(|p| format!("- {}", p)).collect::<Vec<_>>().join("\n"),
            invariants.iter().map(|p| format!("- {}", p)).collect::<Vec<_>>().join("\n"),
        )
    }
}
