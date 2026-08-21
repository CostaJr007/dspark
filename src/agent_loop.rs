//! Multi-turn agentic loop with JSON tool calls.

use crate::client::{ClientError, ModelClient};
use crate::curator::DeepSeekCurator;
use crate::prompts::AGENT_SYSTEM_PROMPT;
use crate::tools::{ToolRegistry, ToolResult};
use regex::Regex;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::OnceLock;

static TOOL_JSON_RE: OnceLock<Regex> = OnceLock::new();

pub struct SparkAgent {
    pub working_dir: PathBuf,
    client: ModelClient,
    tools: ToolRegistry,
}

impl SparkAgent {
    pub fn new(
        working_dir: PathBuf,
        generator_model: &str,
        curator_model: &str,
    ) -> Result<Self, ClientError> {
        let client = ModelClient::from_spec(generator_model)?;
        let curator = DeepSeekCurator::with_model(curator_model).ok();
        Ok(Self {
            tools: ToolRegistry::new(working_dir.clone(), curator),
            working_dir,
            client,
        })
    }

    pub async fn execute_step(
        &self,
        user_prompt: &str,
        on_tool_call: Option<&dyn Fn(&str, &Value)>,
        on_tool_result: Option<&dyn Fn(&ToolResult)>,
        max_iterations: usize,
    ) -> Result<String, ClientError> {
        let mut history = format!("Workspace: {:?}\n\nRequest: {}", self.working_dir, user_prompt);
        let mut last = String::new();

        for _ in 0..max_iterations {
            let response = self
                .client
                .complete(&history, Some(AGENT_SYSTEM_PROMPT), 0.2, false)
                .await?;
            last = response.clone();

            let re = TOOL_JSON_RE.get_or_init(|| {
                Regex::new(r"(?s)```json\s*(\{.*?\})\s*```").expect("tool json")
            });
            let Some(caps) = re.captures(&response) else {
                return Ok(response);
            };

            let Ok(tool_data) = serde_json::from_str::<Value>(&caps[1]) else {
                history.push_str(&format!(
                    "\n\nAssistant:\n{}\n\nUser:\nTool JSON could not be parsed. Continue without the tool.",
                    response
                ));
                continue;
            };

            let tool_name = tool_data
                .get("tool")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let args = tool_data
                .get("args")
                .cloned()
                .unwrap_or(Value::Object(Default::default()));

            if let Some(cb) = on_tool_call {
                cb(tool_name, &args);
            }

            let res = self.dispatch(tool_name, &args).await;
            if let Some(cb) = on_tool_result {
                cb(&res);
            }

            let payload = res.output.clone();
            let err = res.error.clone().unwrap_or_default();
            history.push_str(&format!(
                "\n\nAssistant:\n{}\n\nUser:\nTool '{}' Result (Success: {}):\n{}",
                response,
                tool_name,
                res.success,
                if payload.is_empty() { err } else { payload }
            ));
        }

        Ok(last)
    }

    async fn dispatch(&self, tool_name: &str, args: &Value) -> ToolResult {
        let arg_str = |key: &str| {
            args.get(key)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };
        let arg_i64 = |key: &str| args.get(key).and_then(|v| v.as_i64());

        match tool_name {
            "read_file" => self
                .tools
                .read_file(&arg_str("path"), arg_i64("start_line"), arg_i64("end_line")),
            "write_file" => self.tools.write_file(&arg_str("path"), &arg_str("content")),
            "edit_file" => self.tools.edit_file(
                &arg_str("path"),
                &arg_str("target_chunk"),
                &arg_str("replacement_chunk"),
            ),
            "list_files" => {
                let rel = arg_str("relative_path");
                self.tools
                    .list_files(if rel.is_empty() { "." } else { &rel })
            }
            "run_terminal" => self.tools.run_terminal(&arg_str("command"), 90),
            "search_web" => self.tools.search_web(&arg_str("query")).await,
            "verify_with_curator" => {
                self.tools
                    .verify_with_curator(&arg_str("code"), &arg_str("specification"))
                    .await
            }
            _ => ToolResult {
                tool_name: tool_name.into(),
                success: false,
                output: String::new(),
                error: Some(format!("Unknown tool '{}'", tool_name)),
            },
        }
    }
}
