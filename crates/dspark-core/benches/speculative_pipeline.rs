use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use dspark::engine::confidence_head::{BlockConfidence, ConfidenceHead};
use dspark::engine::cost_scheduler::CostScheduler;
use dspark::engine::speculative_drafter::DraftTrajectory;
use dspark::utils::CodeBlock;

fn run_speculative_pipeline(n_trajectories: usize, blocks_per_trajectory: usize) -> (usize, f64) {
    // 1. Generate synthetic trajectories
    let trajectories: Vec<DraftTrajectory> = (0..n_trajectories)
        .map(|_| DraftTrajectory {
            code_blocks: (0..blocks_per_trajectory)
                .map(|i| CodeBlock {
                    function_name: format!("fn_{}", i),
                    code: "fn example() { for _ in 0..5 { if true { } } }".to_string(),
                    line_count: 3,
                })
                .collect(),
            confidence_score: 0.5,
            ast_valid: true,
            ..Default::default()
        })
        .collect();

    // 2. Confidence estimation
    let head = ConfidenceHead::default();
    let all_blocks: Vec<BlockConfidence> = trajectories
        .iter()
        .flat_map(|t| head.estimate_confidence(t))
        .collect();

    // 3. Cost scheduling
    let scheduler = CostScheduler::new(50, 0.002);
    let plan = scheduler.schedule_verification(&all_blocks);

    let total_blocks = all_blocks.len();
    let verifications_needed = plan.blocks_to_verify.len();
    let saved = total_blocks.saturating_sub(verifications_needed);

    (saved, plan.estimated_cost)
}

fn bench_speculative_vs_naive(c: &mut Criterion) {
    let mut group = c.benchmark_group("speculative_pipeline");

    for n in [3, 5, 10].iter() {
        for blocks in [10, 50, 100].iter() {
            // Speculative with pruning
            group.bench_with_input(
                BenchmarkId::new("speculative_pruning", format!("N={}_B={}", n, blocks)),
                &(*n, *blocks),
                |b, &(n, blocks)| {
                    b.iter(|| {
                        let (saved, cost) = run_speculative_pipeline(n, blocks);
                        black_box((saved, cost));
                    });
                },
            );

            // Naive (no pruning)
            group.bench_with_input(
                BenchmarkId::new("naive_full_verify", format!("N={}_B={}", n, blocks)),
                &(*n, *blocks),
                |b, &(n, blocks)| {
                    b.iter(|| {
                        let total_blocks = n * blocks;
                        let cost = total_blocks as f64 * 0.002;
                        black_box((0, cost));
                    });
                },
            );
        }
    }

    group.finish();
}

fn bench_pruning_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("pruning_efficiency");

    for n in [5, 10, 20].iter() {
        let blocks = 50;
        let (saved, cost) = run_speculative_pipeline(*n, blocks);
        let total = *n * blocks;
        let savings = saved as f64 / total as f64 * 100.0;

        group.bench_with_input(
            BenchmarkId::new("savings", format!("N={}", n)),
            n,
            |b, _| {
                b.iter(|| black_box((savings, cost)));
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_speculative_vs_naive,
    bench_pruning_efficiency
);
criterion_main!(benches);
