use dspark::curator::{AuditResult, CurationVerdict, EdgeCase};
use dspark::search::html_to_markdown;
use dspark::util::{extract_code_blocks, extract_json};

#[test]
fn json_extraction_clean() {
    let raw = r#"{"verdict": "APPROVED", "score": 95, "summary": "Great code."}"#;
    let parsed = extract_json(raw).unwrap();
    assert_eq!(parsed["verdict"], "APPROVED");
    assert_eq!(parsed["score"], 95);
}

#[test]
fn json_extraction_markdown_block() {
    let raw = "Here is the analysis:\n```json\n{\"verdict\": \"NEEDS_REVISION\", \"score\": 60, \"summary\": \"Has edge case.\"}\n```\nHope it helps!";
    let parsed = extract_json(raw).unwrap();
    assert_eq!(parsed["verdict"], "NEEDS_REVISION");
    assert_eq!(parsed["score"], 60);
}

#[test]
fn json_extraction_think_tags() {
    let raw = "<think>reasoning</think>\n{\"verdict\": \"REJECTED\", \"score\": 10}";
    let parsed = extract_json(raw).unwrap();
    assert_eq!(parsed["verdict"], "REJECTED");
}

#[test]
fn code_block_extraction() {
    let raw = "```python\ndef hello():\n    return 'world'\n```";
    assert_eq!(extract_code_blocks(raw), "def hello():\n    return 'world'");
}

#[test]
fn audit_result_approved() {
    let res = AuditResult {
        verdict: CurationVerdict::Approved,
        score: 95,
        summary: "All tests passed".into(),
        criteria_scores: serde_json::json!({}),
        counter_examples: vec![],
        io_contract_analysis: serde_json::json!({}),
        edge_cases: vec![EdgeCase {
            case: "Empty array".into(),
            risk_level: "LOW".into(),
            handled_properly: true,
            remedy: String::new(),
        }],
        complexity: None,
        critical_issues: vec![],
        suggested_improvements: vec![],
        refined_code: None,
        raw_response: String::new(),
    };
    assert!(res.is_approved());
    assert!(!res.must_revise());
    assert_eq!(res.edge_cases.len(), 1);
}

#[test]
fn parse_chat_array_content() {
    let raw = r#"{"choices":[{"message":{"content":[{"type":"text","text":"ok"}]}}]}"#;
    assert_eq!(
        dspark::client::parse_chat_completion_text(raw).unwrap(),
        "ok"
    );
}

#[test]
fn html_cleaner() {
    let raw = r#"
        <html>
            <head><style>body { color: red; }</style></head>
            <body>
                <h1>Documentation Title</h1>
                <p>Here is an explanation of <code>asyncio</code> in Python.</p>
                <pre><code>import asyncio
async def main():
    pass</code></pre>
            </body>
        </html>
        "#;
    let md = html_to_markdown(raw);
    assert!(md.contains("# Documentation Title"), "got: {md}");
    assert!(md.contains("`asyncio`"), "got: {md}");
    assert!(md.contains("```"), "got: {md}");
    assert!(!md.contains("<style>"));
}

#[test]
fn read_file_or_string_literal() {
    let s = dspark::util::read_file_or_string("not-a-real-file.py").unwrap();
    assert_eq!(s, "not-a-real-file.py");
}
