//! Criterion benchmarks for DSpark Speculative Orchestration Engine.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dspark::engine::{
    BlockConfidence, ConfidenceHead, CostScheduler, DraftTrajectory, LogprobExtractor, RiskLevel,
    TokenLogprob,
};
use dspark::utils::{AstResolver, CodeBlock};

fn bench_ast_topological_sort(c: &mut Criterion) {
    let resolver = AstResolver::new();

    let mut blocks = Vec::new();
    for i in 0..50 {
        let caller = format!("func_{}", i);
        let callee = if i > 0 { format!("func_{}();", i - 1) } else { "".to_string() };
        blocks.push(CodeBlock {
            function_name: caller.clone(),
            code: format!("fn {}() {{ {} }}", caller, callee),
            line_count: 1,
        });
    }

    c.bench_function("ast_topological_sort_50_blocks", |b| {
        b.iter(|| {
            let (graph, is_valid) = resolver.resolve(black_box(&blocks));
            assert!(is_valid);
            let sorted = graph.topological_sort();
            black_box(sorted);
        });
    });
}

fn bench_confidence_head_entropy(c: &mut Criterion) {
    let conf_head = ConfidenceHead::default();

    let mut code_blocks = Vec::new();
    for i in 0..20 {
        code_blocks.push(CodeBlock {
            function_name: format!("branch_loop_fn_{}", i),
            code: format!(
                "unsafe fn branch_loop_fn_{}() {{ for _ in 0..10 {{ if true {{ match x {{ _ => () }} }} }} }}",
                i
            ),
            line_count: 3,
        });
    }

    let trajectory = DraftTrajectory {
        id: 0,
        full_code: "simulated".to_string(),
        code_blocks,
        confidence_score: 0.0,
        ast_valid: true,
    };

    c.bench_function("confidence_head_20_blocks", |b| {
        b.iter(|| {
            let scores = conf_head.estimate_confidence(black_box(&trajectory));
            black_box(scores);
        });
    });
}

fn bench_cost_scheduler_pruning(c: &mut Criterion) {
    let scheduler = CostScheduler::new(25, 0.002);

    let mut confidences = Vec::new();
    for i in 0..200 {
        confidences.push(BlockConfidence {
            block_id: i,
            function_name: format!("block_{}", i),
            confidence_score: (i as f64 % 100.0) / 100.0,
            needs_verification: i % 2 == 0,
            risk_level: if i % 3 == 0 { RiskLevel::High } else { RiskLevel::Medium },
        });
    }

    c.bench_function("cost_scheduler_pruning_200_blocks", |b| {
        b.iter(|| {
            let plan = scheduler.schedule_verification(black_box(&confidences));
            black_box(plan);
        });
    });
}

fn bench_logprob_extractor(c: &mut Criterion) {
    let extractor = LogprobExtractor::new();

    let mut logprobs = Vec::new();
    for i in 0..100 {
        logprobs.push(TokenLogprob {
            token: format!("token_{}", i),
            logprob: -0.05 * (i as f64 % 10.0 + 1.0),
            top_logprobs: vec![
                (format!("token_{}", i), -0.05),
                ("alt_token".to_string(), -2.5),
            ],
        });
    }

    c.bench_function("logprob_extractor_100_tokens", |b| {
        b.iter(|| {
            let res = extractor.analyze(black_box("APPROVED: valid implementation"), black_box(&logprobs));
            black_box(res);
        });
    });
}

criterion_group!(
    benches,
    bench_ast_topological_sort,
    bench_confidence_head_entropy,
    bench_cost_scheduler_pruning,
    bench_logprob_extractor
);
criterion_main!(benches);
