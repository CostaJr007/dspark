use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dspark::utils::ast_resolver::{CodeBlock, RegexResolver};

#[cfg(feature = "tree-sitter-ast")]
use dspark::utils::ast_resolver::TreeSitterResolver;

fn make_blocks(n: usize) -> Vec<CodeBlock> {
    (0..n)
        .map(|i| CodeBlock {
            function_name: format!("fn_{}", i),
            code: format!(
                r#"fn fn_{}(x: i32) -> i32 {{
    let y = fn_{}(x);
    let z = fn_{}(y);
    y + z
}}"#,
                i,
                (i + 1) % n,
                (i + 2) % n
            ),
            line_count: 5,
        })
        .collect()
}

fn bench_ast_resolvers(c: &mut Criterion) {
    let mut group = c.benchmark_group("ast_resolver");

    for n in [5, 10, 20, 50, 100].iter() {
        let blocks = make_blocks(*n);
        let regex_resolver = RegexResolver::new();

        group.bench_with_input(
            BenchmarkId::new("regex", format!("N={}", n)),
            n,
            |b, _| {
                b.iter(|| {
                    let _ = regex_resolver.resolve(&blocks, "rust");
                });
            },
        );

        #[cfg(feature = "tree-sitter-ast")]
        {
            let ts_resolver = TreeSitterResolver::new();
            group.bench_with_input(
                BenchmarkId::new("tree-sitter", format!("N={}", n)),
                n,
                |b, _| {
                    b.iter(|| {
                        let _ = ts_resolver.resolve(&blocks, "rust");
                    });
                },
            );
        }
    }

    group.finish();
}

criterion_group!(benches, bench_ast_resolvers);
criterion_main!(benches);
