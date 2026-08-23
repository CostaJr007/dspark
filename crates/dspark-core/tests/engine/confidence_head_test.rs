use dspark::engine::confidence_head::{ConfidenceHead, RiskLevel};
use dspark::engine::speculative_drafter::DraftTrajectory;
use dspark::utils::ast_resolver::CodeBlock;

fn make_block(code: &str, fn_name: &str) -> CodeBlock {
    CodeBlock {
        function_name: fn_name.to_string(),
        code: code.to_string(),
        line_count: 3,
    }
}

#[test]
fn test_detects_complex_logic() {
    let head = ConfidenceHead::new(0.85);

    let trajectory = DraftTrajectory {
        code_blocks: vec![
            make_block("async fn fetch() { unsafe { } }", "fetch"), // Complex
            make_block("fn add(a: i32, b: i32) -> i32 { a + b }", "add"), // Simple
        ],
        confidence_score: 0.0,
        ast_valid: true,
        ..Default::default()
    };

    let confidences = head.estimate_confidence(&trajectory);

    assert!(confidences[0].confidence_score < confidences[1].confidence_score);
    assert!(matches!(
        confidences[0].risk_level,
        RiskLevel::High | RiskLevel::Medium
    ));
}

#[test]
fn test_detects_state_mutation() {
    let head = ConfidenceHead::new(0.85);

    let trajectory = DraftTrajectory {
        code_blocks: vec![
            make_block(
                "fn mutate(state: &mut State) { static mut FOO: i32 = 0; }",
                "mutate",
            ),
            make_block("fn pure_calc(x: i32) -> i32 { x * 2 }", "pure_calc"),
        ],
        confidence_score: 0.0,
        ast_valid: true,
        ..Default::default()
    };

    let confidences = head.estimate_confidence(&trajectory);
    assert!(confidences[0].confidence_score < confidences[1].confidence_score);
}
