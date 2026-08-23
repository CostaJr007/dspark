use dspark::client::LocalLLMClient;
use dspark::engine::pivot_tournament::PivotTournament;
use dspark::engine::speculative_drafter::DraftTrajectory;
use dspark::utils::ast_resolver::CodeBlock;

fn make_trajectory(name: &str) -> DraftTrajectory {
    DraftTrajectory {
        id: 0,
        full_code: format!("fn {}() {{}}", name),
        code_blocks: vec![CodeBlock {
            function_name: name.to_string(),
            code: format!("fn {}() {{}}", name),
            line_count: 1,
        }],
        confidence_score: 0.5,
        ast_valid: true,
    }
}

#[tokio::test]
async fn test_tournament_handles_single_trajectory() {
    let local_client = LocalLLMClient::new(Some("http://127.0.0.1:11434/v1"), Some("qwen")).unwrap();
    let tournament = PivotTournament::new(dspark::client::ModelClient::Local(local_client), 2);
    let trajectories = vec![make_trajectory("only_one")];

    let result = tournament.run_tournament(&trajectories, "test").await;

    assert_eq!(result.best_trajectory_idx, 0);
    assert_eq!(result.total_comparisons, 0); // No comparisons needed for N=1
}

#[tokio::test]
async fn test_total_comparisons_is_onk_not_on2() {
    let n = 10;
    let k = 3;

    let local_client = LocalLLMClient::new(Some("http://127.0.0.1:11434/v1"), Some("qwen")).unwrap();
    let tournament = PivotTournament::new(dspark::client::ModelClient::Local(local_client), k);
    let trajectories: Vec<_> = (0..n).map(|i| make_trajectory(&format!("fn_{}", i))).collect();

    let result = tournament.run_tournament(&trajectories, "test criteria").await;

    // Ring pass: N = 10
    // Pivot tournament: (N-k)*k + k*(k-1)/2 = 7*3 + 3 = 24
    // Total expected = 10 + 24 = 34
    let expected_onk = n + (n - k) * k + k * (k - 1) / 2;
    let on2 = n * (n - 1) / 2; // 45 comparisons

    assert_eq!(result.total_comparisons, expected_onk);
    assert!(
        result.total_comparisons < on2,
        "Tournament should be O(Nk)={} not O(N2)={}",
        result.total_comparisons,
        on2
    );
    assert_eq!(result.rankings.len(), n);
}
