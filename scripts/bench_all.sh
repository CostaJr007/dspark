#!/bin/bash
set -e

echo "🚀 Running all DSpark benchmarks..."

echo ""
echo "=== 1/3: Pivot Tournament (O(Nk) vs O(N²)) ==="
cargo bench --package dspark-core --bench pivot_tournament

echo ""
echo "=== 2/3: AST Resolver (Regex vs Tree-sitter) ==="
cargo bench --package dspark-core --bench ast_resolver --features tree-sitter-ast

echo ""
echo "=== 3/3: Speculative Pipeline ==="
cargo bench --package dspark-core --bench speculative_pipeline

echo ""
echo "✅ All benchmarks complete!"
echo "📊 Reports available at: target/criterion/report/index.html"
