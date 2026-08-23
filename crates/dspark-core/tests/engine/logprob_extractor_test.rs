use dspark::engine::logprob_extractor::{
    LogprobExtractor, TokenLogprob, VerificationVerdict,
};

fn make_logprob(token: &str, logp: f64, tops: Vec<(&str, f64)>) -> TokenLogprob {
    TokenLogprob {
        token: token.to_string(),
        logprob: logp,
        top_logprobs: tops.into_iter().map(|(t, p)| (t.to_string(), p)).collect(),
    }
}

#[test]
fn test_high_confidence_low_entropy_approves() {
    let extractor = LogprobExtractor::new();
    let logprobs = vec![make_logprob(
        "APPROVED",
        -0.05,
        vec![("APPROVED", -0.05), ("REJECTED", -5.0)],
    )];

    let result = extractor.analyze("APPROVED: code is formally correct", &logprobs);

    assert_eq!(result.verdict, VerificationVerdict::Approved);
    assert!(result.confidence > 0.90);
    assert!(result.entropy < 0.50);
}

#[test]
fn test_high_entropy_marks_uncertain() {
    let extractor = LogprobExtractor::new();
    // Uniform spread = high entropy
    let logprobs = vec![make_logprob(
        "x",
        -1.38,
        vec![("a", -1.38), ("b", -1.38), ("c", -1.38), ("d", -1.38)],
    )];

    let result = extractor.analyze("Uncertain evaluation", &logprobs);
    assert!(result.entropy > 0.80);
    assert!(matches!(result.verdict, VerificationVerdict::Rejected(_)));
}

#[test]
fn test_fine_grained_reward_formula() {
    let extractor = LogprobExtractor::new();
    let logprobs = vec![make_logprob(
        "excellent",
        -0.01,
        vec![("excellent", -0.01), ("good", -5.0), ("poor", -10.0)],
    )];

    let result = extractor.analyze("APPROVED: excellent", &logprobs);
    assert!(result.fine_grained_reward > 0.90);
}
