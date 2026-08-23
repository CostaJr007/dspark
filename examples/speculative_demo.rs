//! Example: Running local speculative confidence estimation and tournament logic in Rust.

use dspark::engine::{ConfidenceHead, CostScheduler, DraftTrajectory, PivotTournament};
use dspark::utils::ast_resolver::CodeBlock;

#[tokio::main]
async fn main() {
    println!("=== DSpark Speculative Engine Demonstration ===");

    // 1. Create simulated code trajectories
    let mut trajectories = Vec::new();
    for i in 0..4 {
        trajectories.push(DraftTrajectory {
            id: i,
            full_code: format!("fn solve_problem_{}() -> i32 {{ {} * 2 }}", i, i),
            code_blocks: vec![CodeBlock {
                function_name: format!("solve_problem_{}", i),
                code: format!("fn solve_problem_{}() -> i32 {{ {} * 2 }}", i, i),
                line_count: 1,
            }],
            confidence_score: 0.85,
            ast_valid: true,
        });
    }

    // 2. Estimate confidence and local entropy
    let head = ConfidenceHead::default();
    let confs = head.estimate_confidence(&trajectories[0]);
    println!("Trajectory #0 Risk Level: {:?}", confs[0].risk_level);

    // 3. Schedule verification
    let scheduler = CostScheduler::new(10, 0.002);
    let plan = scheduler.schedule_verification(&confs);
    println!("Scheduled {} verifications. Est cost: ${:.4}", plan.blocks_to_verify.len(), plan.estimated_cost);

    println!("Demo completed successfully!");
}
