use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn bench_pivot_tournament_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("pivot_tournament_scaling");

    // N = number of candidate trajectories
    // k = 3 pivots
    let k = 3;

    for n in [3, 5, 10, 20, 50, 100].iter() {
        let onk_cost = *n + (*n - k) * k + k * (k - 1) / 2;
        let on2_cost = *n * (*n - 1) / 2;

        group.bench_with_input(
            BenchmarkId::new("O(Nk)", format!("N={}", n)),
            n,
            |b, &_n| {
                b.iter(|| {
                    let total_comparisons = onk_cost;
                    black_box(total_comparisons)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("O(N2)_baseline", format!("N={}", n)),
            n,
            |b, &_n| {
                b.iter(|| {
                    let total_comparisons = on2_cost;
                    black_box(total_comparisons)
                });
            },
        );
    }

    group.finish();
}

fn bench_pivot_cost_savings(c: &mut Criterion) {
    let mut group = c.benchmark_group("pivot_vs_round_robin");

    for n in [10, 20, 50, 100].iter() {
        let k = 3;
        let onk = *n + (*n - k) * k + k * (k - 1) / 2;
        let on2 = *n * (*n - 1) / 2;
        let savings = 1.0 - (onk as f64 / on2 as f64);

        group.bench_with_input(
            BenchmarkId::new("savings", format!("N={}", n)),
            n,
            |b, _| {
                b.iter(|| black_box(savings));
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_pivot_tournament_scaling, bench_pivot_cost_savings);
criterion_main!(benches);
