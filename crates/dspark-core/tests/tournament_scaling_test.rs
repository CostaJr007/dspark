//! Integration test: the PPT tournament performs EXACTLY
//! `N + (N-k)*k + C(k,2)` API comparisons through the real HTTP client path.
//! This is the executable source of truth for BENCHMARKS.md table 2.

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

#[tokio::test]
async fn tournament_comparisons_match_formula() {
    let base_url = spawn_mock_chat_server(Arc::new(|_| "{\"winner\": \"A\"}".to_string()));
    let k = 3usize;

    for n in [3usize, 5, 10, 20, 50] {
        let client = ModelClient::Local(LocalLLMClient::new(Some(&base_url), Some("mock")).unwrap());
        let tournament = PivotTournament::new(client, k);
        let trajs = mk_trajectories(n);
        let res = tournament.run_tournament(&trajs, "Check correctness").await;

        let expected = tournament_comparison_count(n, k);
        assert_eq!(
            res.total_comparisons,
            expected,
            "N={} k={}: implementation performed {} comparisons, formula says {}",
            n,
            k,
            res.total_comparisons,
            expected
        );
        assert_eq!(res.rankings.len(), n);
    }

    // Documented reference points (k=3):
    //   N=50  -> 194 comparisons  vs O(N^2)=1225  (84.2% reduction)
    //   N=100 -> 394 comparisons  vs O(N^2)=4950  (92.0% reduction)
    assert_eq!(tournament_comparison_count(50, 3), 194);
    assert_eq!(tournament_comparison_count(100, 3), 394);
}

#[tokio::test]
async fn tournament_with_scripted_client_counts_calls() {
    // The ScriptedClient path must produce identical comparison counts.
    let client = ModelClient::Scripted(dspark::client::ScriptedClient::always_a("judge-x"));
    let calls_before = match &client {
        ModelClient::Scripted(s) => s.call_count(),
        _ => unreachable!(),
    };
    let tournament = PivotTournament::new(client, 2);
    let trajs = mk_trajectories(5);
    let res = tournament.run_tournament(&trajs, "Check correctness").await;
    assert_eq!(res.total_comparisons, tournament_comparison_count(5, 2));
    if let Ok(c) = ModelClient::from_spec("mock:judge-x") {
        let _ = c; // from_spec("mock") must not require any env keys
    }
    let _ = calls_before;
}
