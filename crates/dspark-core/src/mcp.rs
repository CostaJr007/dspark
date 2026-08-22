//! Model Context Protocol server (stdio JSON-RPC).

use crate::curator::DeepSeekCurator;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

const TOOLS: &str = r#"[
  {
    "name": "dspark_audit_code",
    "description": "Perform deep reasoning analysis and I/O contract arbitration on candidate code using DeepSeek.",
    "inputSchema": {
      "type": "object",
      "properties": {
        "code": {"type": "string", "description": "The candidate code to audit."},
        "specification": {"type": "string", "description": "The requirements, task prompt, or I/O contract specification."},
        "language": {"type": "string", "description": "Programming language (e.g. python, typescript, rust, c++)."}
      },
      "required": ["code", "specification"]
    }
  },
  {
    "name": "dspark_refine_code",
    "description": "Refine code using DeepSeek to ensure 100% production readiness, strict typing, and edge case coverage.",
    "inputSchema": {
      "type": "object",
      "properties": {
        "code": {"type": "string", "description": "Draft code to optimize/fix."},
        "specification": {"type": "string", "description": "Requirements or goals."},
        "feedback": {"type": "string", "description": "Specific audit feedback or issues to fix."},
        "language": {"type": "string", "description": "Programming language."}
      },
      "required": ["code", "specification"]
    }
  },
  {
    "name": "dspark_arbitrate",
    "description": "Arbitrate between two or more alternative code implementations and synthesize the optimal solution.",
    "inputSchema": {
      "type": "object",
      "properties": {
        "candidates": {
          "type": "array",
          "items": {"type": "string"},
          "description": "List of candidate implementations to compare."
        },
        "specification": {"type": "string", "description": "The requirements to evaluate against."},
        "language": {"type": "string", "description": "Programming language."}
      },
      "required": ["candidates", "specification"]
    }
  }
]"#;

pub async fn run_mcp_server() -> Result<(), Box<dyn std::error::Error>> {
    let curator = DeepSeekCurator::new()?;
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut stdout = tokio::io::stdout();

    loop {
        let req = match read_message(&mut reader).await? {
            Some(v) => v,
            None => break,
        };
        let req_id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

        let resp = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "dspark-mcp", "version": "0.1.0" }
                }
            }),
            "notifications/initialized" | "initialized" => continue,
            "tools/list" => {
                let tools: Value = serde_json::from_str(TOOLS)?;
                json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": { "tools": tools }
                })
            }
            "tools/call" => {
                let params = req.get("params").cloned().unwrap_or(json!({}));
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                match handle_tool_call(&curator, tool_name, &args).await {
                    Ok(payload) => json!({
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "result": {
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&payload).unwrap_or_default()
                            }]
                        }
                    }),
                    Err(e) => json!({
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "error": { "code": -32000, "message": e }
                    }),
                }
            }
            _ => json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "error": { "code": -32601, "message": format!("Method {method} not supported") }
            }),
        };

        write_message(&mut stdout, &resp).await?;
    }

    Ok(())
}

async fn handle_tool_call(
    curator: &DeepSeekCurator,
    name: &str,
    arguments: &Value,
) -> Result<Value, String> {
    let str_arg = |key: &str| {
        arguments
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let opt_lang = arguments
        .get("language")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());

    match name {
        "dspark_audit_code" => {
            let result = curator
                .audit(&str_arg("code"), &str_arg("specification"), opt_lang)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({
                "verdict": result.verdict.as_str(),
                "score": result.score,
                "summary": result.summary,
                "io_contract_analysis": result.io_contract_analysis,
                "edge_cases": result.edge_cases,
                "complexity": result.complexity,
                "critical_issues": result.critical_issues,
                "suggested_improvements": result.suggested_improvements,
                "refined_code": result.refined_code,
            }))
        }
        "dspark_refine_code" => {
            let feedback = arguments.get("feedback").and_then(|v| v.as_str());
            let res = curator
                .refine(
                    &str_arg("code"),
                    &str_arg("specification"),
                    feedback,
                    opt_lang,
                )
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({
                "refined_code": res.refined_code,
                "summary_of_changes": res.summary_of_changes,
            }))
        }
        "dspark_arbitrate" => {
            let candidates: Vec<String> = arguments
                .get("candidates")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let res = curator
                .arbitrate(&candidates, &str_arg("specification"), opt_lang)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({
                "winner_index": res.winner_index,
                "rationale": res.rationale,
                "comparison_matrix": res.comparison_matrix,
                "synthesized_code": res.synthesized_code,
            }))
        }
        _ => Err(format!("Unknown tool name: {}", name)),
    }
}

async fn read_message(
    reader: &mut BufReader<tokio::io::Stdin>,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    let mut header = String::new();
    let n = reader.read_line(&mut header).await?;
    if n == 0 {
        return Ok(None);
    }
    let header_trim = header.trim();
    if header_trim.is_empty() {
        return Ok(None);
    }

    if header_trim.starts_with('{') {
        return Ok(Some(serde_json::from_str(header_trim)?));
    }

    if header.to_ascii_lowercase().starts_with("content-length:") {
        let mut content_length = header
            .split(':')
            .nth(1)
            .and_then(|s| s.trim().parse::<usize>().ok())
            .unwrap_or(0);
        loop {
            let mut line = String::new();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                return Ok(None);
            }
            if line.trim().is_empty() {
                break;
            }
            if line.to_ascii_lowercase().starts_with("content-length:") {
                content_length = line
                    .split(':')
                    .nth(1)
                    .and_then(|s| s.trim().parse::<usize>().ok())
                    .unwrap_or(content_length);
            }
        }
        let mut buf = vec![0u8; content_length];
        reader.read_exact(&mut buf).await?;
        return Ok(Some(serde_json::from_slice(&buf)?));
    }

    Ok(Some(serde_json::from_str(header_trim)?))
}

async fn write_message(
    stdout: &mut tokio::io::Stdout,
    value: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let body = serde_json::to_string(value)?;
    let framed = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    stdout.write_all(framed.as_bytes()).await?;
    stdout.flush().await?;
    Ok(())
}
