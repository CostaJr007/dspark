use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use dspark::client::{spawn_mock_chat_server, LocalLLMClient, ModelClient};
use dspark::engine::{tournament_comparison_count, DraftTrajectory, PivotTournament};
use dspark::utils::ast_resolver::CodeBlock;
use std::sync::Arc;

fn mk_trajectories(n: usize) -> Vec<DraftTrajectory> {
    (0..n)
        .map(|i| DraftTrajectory {
            id: i,
            full_code: format!("fn candidate_{}() {{ {} }}", i, i),
            code_blocks: vec![CodeBlock {
                function_name: format!("candidate_{}", i),
                code: format!("fn candidate_{}() {{ {} }}", i, i),
                line_count: 1,
            }],
            confidence_score: 0.8,
            ast_valid: true,
        })
        .collect()
}

/// End-to-end benchmark: runs the FULL PPT algorithm (ring pass + pivot
/// tournament) through the real HTTP client path against a local mock
/// OpenAI-compatible server. Every iteration performs exactly
/// `N + (N-k)*k + C(k,2)` network round trips, so the measurement reflects the
/// actual orchestration cost instead of precomputed arithmetic.
fn bench_pivot_tournament_e2e(c: &mut Criterion) {
    let base_url = spawn_mock_chat_server(Arc::new(|_| "{\"winner\": \"A\"}".to_string()));
    let rt = tokio::runtime::Runtime::new().unwrap();
    let k = 3usize;

    let mut group = c.benchmark_group("pivot_tournament_e2e");
    // N is capped at 20 here because each iteration performs N + (N-k)*k + C(k,2)
    // sequentializable HTTP round trips through a single-threaded mock server.
    for n in [3usize, 5, 10, 20].iter() {
        let expected = tournament_comparison_count(*n, k);
        group.bench_with_input(
            BenchmarkId::new("full_tournament", format!("N={}", n)),
            n,
            |b, &_n| {
                b.iter(|| {
                    let client =
                        ModelClient::Local(LocalLLMClient::new(Some(&base_url), Some("mock")).unwrap());
                    let tournament = PivotTournament::new(client, k);
                    let trajs = mk_trajectories(*n);
                    let res = rt.block_on(tournament.run_tournament(&trajs, "Check correctness"));
                    assert_eq!(res.total_comparisons, black_box(expected));
                    res.best_trajectory_idx
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_pivot_tournament_e2e);
criterion_main!(benches);
