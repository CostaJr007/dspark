//! Comprehensive integration and unit tests for the DSpark Speculative Orchestration Engine.

use dspark::engine::{
    BlockConfidence, ConfidenceHead, CostScheduler, DraftTrajectory, LogprobExtractor, RiskLevel,
    TokenLogprob, VerificationVerdict,
};
use dspark::utils::{AstResolver, CodeBlock, PromptOptimizer};

#[test]
fn test_ast_resolver_topological_sort() {
    let resolver = AstResolver::new();

    // Helper block depends on nothing; Main block calls helper()
    let block_helper = CodeBlock {
        function_name: "helper".to_string(),
        code: "fn helper() -> i32 { 42 }".to_string(),
        line_count: 1,
    };
    let block_main = CodeBlock {
        function_name: "main_func".to_string(),
        code: "fn main_func() { let x = helper(); println!(\"{}\", x); }".to_string(),
        line_count: 1,
    };

    let (dep_graph, is_valid) = resolver.resolve(&[block_main.clone(), block_helper.clone()]);
    assert!(is_valid);
    assert!(!dep_graph.has_cycle());

    let sorted = dep_graph.topological_sort();
    assert_eq!(sorted.len(), 2);
    // Dependency (helper) must come before caller (main_func)
    assert_eq!(sorted[0].function_name, "helper");
    assert_eq!(sorted[1].function_name, "main_func");
}

#[test]
fn test_ast_resolver_split_into_blocks() {
    let resolver = AstResolver::new();
    let sample_code = r#"
fn func_one() {
    println!("one");
}

fn func_two() {
    func_one();
}
"#;
    let blocks = resolver.split_into_blocks(sample_code);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].function_name, "func_one");
    assert_eq!(blocks[1].function_name, "func_two");
}

#[test]
fn test_confidence_head_heuristics() {
    let conf_head = ConfidenceHead::default();

    let simple_traj = DraftTrajectory {
        id: 0,
        full_code: "fn simple() { 1 + 1; }".to_string(),
        code_blocks: vec![CodeBlock {
            function_name: "simple".to_string(),
            code: "fn simple() { 1 + 1; }".to_string(),
            line_count: 1,
        }],
        confidence_score: 0.0,
        ast_valid: true,
    };

    let complex_traj = DraftTrajectory {
        id: 1,
        full_code: "unsafe fn complex_mut() { for _ in 0..10 { if true { match x { _ => () } } } }".to_string(),
        code_blocks: vec![CodeBlock {
            function_name: "complex_mut".to_string(),
            code: "unsafe fn complex_mut() { for _ in 0..10 { if true { match x { _ => () } } } }".to_string(),
            line_count: 1,
        }],
        confidence_score: 0.0,
        ast_valid: true,
    };

    let simple_confs = conf_head.estimate_confidence(&simple_traj);
    let complex_confs = conf_head.estimate_confidence(&complex_traj);

    assert_eq!(simple_confs[0].risk_level, RiskLevel::Low);
    assert!(!simple_confs[0].needs_verification);

    assert!(complex_confs[0].confidence_score < simple_confs[0].confidence_score);
    assert!(complex_confs[0].needs_verification);
}

#[test]
fn test_cost_scheduler_pruning() {
    let scheduler = CostScheduler::new(2, 0.002);

    let confs = vec![
        BlockConfidence {
            block_id: 0,
            function_name: "low_risk_fn".to_string(),
            confidence_score: 0.95,
            needs_verification: false,
            risk_level: RiskLevel::Low,
        },
        BlockConfidence {
            block_id: 1,
            function_name: "high_risk_fn_1".to_string(),
            confidence_score: 0.40,
            needs_verification: true,
            risk_level: RiskLevel::High,
        },
        BlockConfidence {
            block_id: 2,
            function_name: "high_risk_fn_2".to_string(),
            confidence_score: 0.30,
            needs_verification: true,
            risk_level: RiskLevel::High,
        },
        BlockConfidence {
            block_id: 3,
            function_name: "med_risk_fn".to_string(),
            confidence_score: 0.70,
            needs_verification: true,
            risk_level: RiskLevel::Medium,
        },
    ];

    let plan = scheduler.schedule_verification(&confs);
    assert_eq!(plan.total_blocks, 4);
    assert_eq!(plan.blocks_to_verify.len(), 2); // Max 2 calls
    assert_eq!(plan.pruned_blocks_count, 2);
    // Highest risk (block 2 and block 1) prioritized
    assert_eq!(plan.blocks_to_verify[0], 2);
    assert_eq!(plan.blocks_to_verify[1], 1);
    assert!(plan.estimated_cost <= 0.004 + 1e-6);
}

#[test]
fn test_logprob_extractor_reward_and_entropy() {
    let extractor = LogprobExtractor::new();

    let logprobs = vec![
        TokenLogprob {
            token: "APPROVED".to_string(),
            logprob: -0.05,
            top_logprobs: vec![("APPROVED".to_string(), -0.05), ("REJECTED".to_string(), -3.0)],
        },
        TokenLogprob {
            token: "correct".to_string(),
            logprob: -0.10,
            top_logprobs: vec![("correct".to_string(), -0.10), ("flaw".to_string(), -2.5)],
        },
    ];

    let result = extractor.analyze("APPROVED: code is correct", &logprobs);
    assert_eq!(result.verdict, VerificationVerdict::Approved);
    assert!(result.confidence > 0.85);
    assert!(result.entropy < 0.5);
    assert!(result.fine_grained_reward > 0.90);
}

#[test]
fn test_prompt_optimizer_prefix_cache_format() {
    let optimizer = PromptOptimizer::new();
    let block_a = CodeBlock {
        function_name: "a".to_string(),
        code: "fn a() {}".to_string(),
        line_count: 1,
    };
    let block_b = CodeBlock {
        function_name: "b".to_string(),
        code: "fn b() {}".to_string(),
        line_count: 1,
    };

    let prompt = optimizer.generate_comparison_prompt(&[block_a], &[block_b], "Check bounds");
    // Code should precede criteria
    assert!(prompt.find("Candidate A Implementation").unwrap() < prompt.find("Comparative Evaluation Criteria").unwrap());
    assert!(prompt.contains("Check bounds"));
}
