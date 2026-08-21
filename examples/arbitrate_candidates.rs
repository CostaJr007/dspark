//! Example 02: Arbitrate between two candidate implementations.

use dspark::DeepSeekCurator;

const CANDIDATE_A: &str = r#"
def fibonacci(n: int) -> int:
    if n <= 0:
        return 0
    elif n == 1:
        return 1
    return fibonacci(n - 1) + fibonacci(n - 2)
"#;

const CANDIDATE_B: &str = r#"
def fibonacci(n: int) -> int:
    if n <= 0:
        return 0
    if n == 1:
        return 1
    a, b = 0, 1
    for _ in range(2, n + 1):
        a, b = b, a + b
    return b
"#;

const SPECIFICATION: &str =
    "Calculate the N-th Fibonacci number efficiently for n up to 100,000 with O(1) auxiliary space.";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let curator = DeepSeekCurator::new()?;
    println!("Arbitrating between Candidate A (Recursive) and Candidate B (Iterative)...");
    let result = curator
        .arbitrate(
            &[CANDIDATE_A.to_string(), CANDIDATE_B.to_string()],
            SPECIFICATION,
            Some("python"),
        )
        .await?;

    println!("\nWinning Candidate Index: #{}", result.winner_index);
    println!("Rationale: {}\n", result.rationale);
    println!("Synthesized Optimal Code:");
    println!("{}", result.synthesized_code);
    Ok(())
}
