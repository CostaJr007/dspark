//! Prompt-contract tests: properties the theory REQUIRES but no test asserted.
//!
//! 1. Epistemic isolation (THEORY.md Sec. 1): the curator's outgoing prompts must
//!    contain only code + specification (+ audit feedback). A creator reasoning
//!    trace that exists in scope must never leak into them. This is structural:
//!    `audit`/`refine` do not even accept a trace parameter; the test pins that.
//! 2. Reward-decomposition weights (THEORY.md Sec. 4): the documented 35/35/30
//!    split must be present in the verifier/curator prompts, and the
//!    `criteria_scores` channel those weights govern must flow through parsing.

use dspark::client::{ModelClient, ScriptedClient};
use dspark::curator::DeepSeekCurator;
use dspark::prompts::CURATOR_SYSTEM_PROMPT;
use dspark::verifier::VERIFIER_SYSTEM_PROMPT;
use std::sync::{Arc, Mutex};

const CREATOR_TRACE_SENTINEL: &str = "CREATOR_TRACE_SENTINEL_7Q9X_REASONING_MUST_NOT_LEAK";

fn capturing_client(
    log: Arc<Mutex<Vec<String>>>,
    response: &'static str,
) -> ModelClient {
    ModelClient::Scripted(ScriptedClient::new("contract-probe", move |prompt| {
        log.lock().unwrap().push(prompt.to_string());
        response.to_string()
    }))
}

const AUDIT_OK: &str = r#"{
    "verdict": "APPROVED",
    "score": 95,
    "summary": "clean",
    "criteria_scores": {"specification": 90, "io_contract": 92, "performance": 88}
}"#;

#[tokio::test]
async fn curator_prompts_exclude_creator_trace() {
    // The trace exists in this scope (as it would in a real agent loop) but is
    // never passed to the curator. If any future refactor adds a trace/draft
    // parameter and forwards it, this test fails.
    let _creator_trace_holder = CREATOR_TRACE_SENTINEL.to_string();

    let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
    let spec = "add returns the sum of two integers";

    let log = Arc::new(Mutex::new(Vec::new()));
    let curator = DeepSeekCurator::with_client(capturing_client(Arc::clone(&log), AUDIT_OK));
    let audit = curator
        .audit(code, spec, Some("rust"))
        .await
        .expect("scripted audit must succeed");
    assert!(audit.is_approved());
    // The criteria_scores channel the weights govern must survive parsing.
    assert_eq!(
        audit.criteria_scores.get("specification").and_then(|x| x.as_u64()),
        Some(90)
    );
    assert_eq!(
        audit.criteria_scores.get("io_contract").and_then(|x| x.as_u64()),
        Some(92)
    );
    assert_eq!(
        audit.criteria_scores.get("performance").and_then(|x| x.as_u64()),
        Some(88)
    );

    let log2 = Arc::new(Mutex::new(Vec::new()));
    let refine_resp = "```rust\nfn add(a: i32, b: i32) -> i32 { a + b }\n```";
    let curator2 = DeepSeekCurator::with_client(capturing_client(Arc::clone(&log2), refine_resp));
    let _ = curator2
        .refine(code, spec, Some("no issues"), Some("rust"))
        .await
        .expect("scripted refine must succeed");

    let prompts: Vec<String> = log
        .lock()
        .unwrap()
        .iter()
        .chain(log2.lock().unwrap().iter())
        .cloned()
        .collect();
    assert!(
        prompts.len() >= 2,
        "capture must observe audit + refine prompts (got {})",
        prompts.len()
    );
    for p in &prompts {
        assert!(
            !p.contains(CREATOR_TRACE_SENTINEL),
            "creator trace leaked into curator prompt"
        );
        assert!(
            p.contains("fn add"),
            "captured prompt must carry the candidate code (capture vacuous?)"
        );
    }
}

#[test]
fn reward_weights_are_pinned_in_prompts_and_parsed() {
    for (name, prompt) in [
        ("verifier", VERIFIER_SYSTEM_PROMPT),
        ("curator", CURATOR_SYSTEM_PROMPT),
    ] {
        assert!(
            prompt.matches("35%").count() >= 2,
            "{name} prompt must pin the two 35% weights"
        );
        assert!(
            prompt.contains("30%"),
            "{name} prompt must pin the 30% weight"
        );
    }

    // The criteria channel above is exercised end-to-end in
    // `curator_prompts_exclude_creator_trace` (parsed from a real audit call).
    let v: serde_json::Value = serde_json::from_str(AUDIT_OK).unwrap();
    assert!(v.get("criteria_scores").is_some());
}
