//! Example 01: Audit candidate code against a specification with the DeepSeek Curator.

use dspark::DeepSeekCurator;

const SAMPLE_CODE: &str = r#"
def find_median_sorted_arrays(nums1, nums2):
    merged = sorted(nums1 + nums2)
    n = len(merged)
    if n % 2 == 1:
        return float(merged[n // 2])
    else:
        return (merged[n // 2 - 1] + merged[n // 2]) / 2.0
"#;

const SPECIFICATION: &str = r#"
Given two sorted arrays nums1 and nums2 of size m and n respectively,
return the median of the two sorted arrays.
The overall run time complexity must be O(log (m+n)).
Empty arrays should be handled gracefully.
"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing DSpark DeepSeek Curator...");
    let curator = DeepSeekCurator::new()?;

    println!("\nAuditing draft implementation against O(log(m+n)) requirement...");
    let result = curator
        .audit(SAMPLE_CODE, SPECIFICATION, Some("python"))
        .await?;

    println!(
        "\nVerdict: {} (Score: {}/100)",
        result.verdict, result.score
    );
    println!("Summary: {}", result.summary);
    if let Some(cx) = &result.complexity {
        println!("Time Complexity identified: {}", cx.time);
        println!("Optimal: {}", cx.optimal);
    }

    if !result.critical_issues.is_empty() {
        println!("\nCritical Issues Flagged by DeepSeek:");
        for issue in &result.critical_issues {
            println!(" - {}", issue);
        }
    }

    if let Some(code) = result.refined_code {
        println!("\n--- DeepSeek Refined O(log(m+n)) Implementation ---");
        println!("{}", code);
    }

    Ok(())
}
