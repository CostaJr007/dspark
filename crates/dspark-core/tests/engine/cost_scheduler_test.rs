use dspark::engine::confidence_head::{BlockConfidence, RiskLevel};
use dspark::engine::cost_scheduler::CostScheduler;

#[test]
fn test_respects_max_api_calls_budget() {
    let scheduler = CostScheduler::new(3, 0.001); // Max 3 calls

    let blocks = vec![
        BlockConfidence {
            block_id: 0,
            function_name: "f0".to_string(),
            confidence_score: 0.2,
            needs_verification: true,
            risk_level: RiskLevel::High,
        },
        BlockConfidence {
            block_id: 1,
            function_name: "f1".to_string(),
            confidence_score: 0.5,
            needs_verification: true,
            risk_level: RiskLevel::Medium,
        },
        BlockConfidence {
            block_id: 2,
            function_name: "f2".to_string(),
            confidence_score: 0.6,
            needs_verification: true,
            risk_level: RiskLevel::Medium,
        },
        BlockConfidence {
            block_id: 3,
            function_name: "f3".to_string(),
            confidence_score: 0.8,
            needs_verification: true,
            risk_level: RiskLevel::Medium,
        },
        BlockConfidence {
            block_id: 4,
            function_name: "f4".to_string(),
            confidence_score: 0.95,
            needs_verification: false,
            risk_level: RiskLevel::Low,
        },
    ];

    let plan = scheduler.schedule_verification(&blocks);

    assert!(plan.blocks_to_verify.len() <= 3);
    // Prioritizes High risk (block 0)
    assert!(plan.blocks_to_verify.contains(&0));
    // Low risk block 4 is pruned
    assert!(!plan.blocks_to_verify.contains(&4));
}

#[test]
fn test_prioritizes_high_risk_blocks() {
    let scheduler = CostScheduler::new(1, 0.001);

    let blocks = vec![
        BlockConfidence {
            block_id: 0,
            function_name: "f0".to_string(),
            confidence_score: 0.9,
            needs_verification: true,
            risk_level: RiskLevel::Low,
        },
        BlockConfidence {
            block_id: 1,
            function_name: "f1".to_string(),
            confidence_score: 0.3,
            needs_verification: true,
            risk_level: RiskLevel::High, // High Priority
        },
        BlockConfidence {
            block_id: 2,
            function_name: "f2".to_string(),
            confidence_score: 0.7,
            needs_verification: true,
            risk_level: RiskLevel::Medium,
        },
    ];

    let plan = scheduler.schedule_verification(&blocks);
    assert_eq!(plan.blocks_to_verify, vec![1]);
}
