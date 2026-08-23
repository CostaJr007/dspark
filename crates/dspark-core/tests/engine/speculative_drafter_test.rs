use dspark::client::LocalLLMClient;
use dspark::engine::speculative_drafter::{DraftTrajectory, SpeculativeDrafter};
use dspark::utils::ast_resolver::CodeBlock;

#[tokio::test]
async fn test_generates_n_trajectories_structure() {
    let local_client = LocalLLMClient::new(Some("http://127.0.0.1:11434/v1"), Some("qwen")).unwrap();
    let drafter = SpeculativeDrafter::new(dspark::client::ModelClient::Local(local_client), 3);

    let trajectories = vec![
        DraftTrajectory {
            id: 0,
            full_code: "fn a() {}".to_string(),
            code_blocks: vec![CodeBlock {
                function_name: "a".to_string(),
                code: "fn a() {}".to_string(),
                line_count: 1,
            }],
            confidence_score: 0.9,
            ast_valid: true,
        },
        DraftTrajectory {
            id: 1,
            full_code: "".to_string(),
            code_blocks: vec![],
            confidence_score: 0.0,
            ast_valid: false, // Invalid
        },
    ];

    let valid = drafter.apply_sequential_module(trajectories);
    assert_eq!(valid.len(), 1);
    assert_eq!(valid[0].id, 0);
}
