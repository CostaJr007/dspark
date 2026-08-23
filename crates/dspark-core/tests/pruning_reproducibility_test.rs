//! Reproducible source of BENCHMARKS.md table 3 (local pruning efficiency).
//! Uses a deterministic mixed-complexity block generator so every number in
//! the docs can be regenerated with:
//!   cargo test -p dspark-core --test pruning_reproducibility_test -- --nocapture

use dspark::engine::{ConfidenceHead, CostScheduler, DraftTrajectory};
use dspark::utils::ast_resolver::CodeBlock;

/// Cycles through five archetypes of increasing complexity so block
/// confidences vary realistically instead of being all identical.
fn archetype(i: usize) -> &'static str {
    [
        "fn simple_get() -> i32 { 42 }",
        "fn parse(s: &str) -> Option<i32> { s.trim().parse::<i32>().ok() }",
        "fn mutate(v: i32) { let mut acc = 0; acc += v; }",
        "fn complex(x: i32) -> i32 { for _ in 0..10 { if x > 0 { match x { 1 => return 1, 2 => return 2, _ => {} } } } x }",
        "unsafe fn raw(p: *mut u8) { if !p.is_null() { *p = 1; } }",
    ][i % 5]
}

fn mk_trajectory(id: usize, blocks: usize) -> DraftTrajectory {
    DraftTrajectory {
        id,
        full_code: String::new(),
        code_blocks: (0..blocks)
            .map(|i| CodeBlock {
                function_name: format!("fn_{}", i),
                code: archetype(i).to_string(),
                line_count: 3,
            })
            .collect(),
        confidence_score: 0.5,
        ast_valid: true,
    }
}

#[test]
fn pruning_matches_documented_methodology() {
    let head = ConfidenceHead::default();
    let scheduler = CostScheduler::default();

    for (n, b) in [(3usize, 10usize), (5, 50), (10, 100)] {
        let trajs: Vec<DraftTrajectory> = (0..n).map(|i| mk_trajectory(i, b)).collect();
        let all_confs: Vec<_> = trajs
            .iter()
            .flat_map(|t| head.estimate_confidence(t))
            .collect();
        let plan = scheduler.schedule_verification(&all_confs);
        let total = all_confs.len();
        let pruned_pct = plan.pruned_blocks_count as f64 / total as f64 * 100.0;
        println!(
            "PRUNING_TABLE N={:<3} B={:<4} total_blocks={:<5} remote_verifications={:<4} pruned={:<5} ({:.1}%)",
            n, b, total, plan.blocks_to_verify.len(), plan.pruned_blocks_count, pruned_pct
        );
        // Sanity: mixed archetypes must yield partial (neither 0% nor 100%) pruning.
        assert!(plan.pruned_blocks_count > 0, "expected some pruning for N={} B={}", n, b);
        assert!(
            plan.pruned_blocks_count < total,
            "expected some verifications for N={} B={}",
            n,
            b
        );
    }
}
