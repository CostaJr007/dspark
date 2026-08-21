//! Autonomous Coding Agent Runtime in Rust.

use crate::client::{ClientError, DeepSeekClient};
use crate::prompts::METACOGNITIVE_ENGINEERING_PROMPT;
use crate::search::{SearchError, WebSearchEngine};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Client error: {0}")]
    Client(#[from] ClientError),
    #[error("Search error: {0}")]
    Search(#[from] SearchError),
}

pub struct DSparkAgent {
    pub working_dir: PathBuf,
    client: DeepSeekClient,
    pub search_engine: WebSearchEngine,
}

impl DSparkAgent {
    pub fn new(working_dir: Option<PathBuf>) -> Result<Self, AgentError> {
        let wd = working_dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        Ok(Self {
            working_dir: wd,
            client: DeepSeekClient::new()?,
            search_engine: WebSearchEngine::new(),
        })
    }

    pub fn read_file<P: AsRef<Path>>(&self, path: P) -> Result<String, AgentError> {
        let full_path = self.working_dir.join(path);
        let content = fs::read_to_string(full_path)?;
        Ok(content)
    }

    pub fn write_file<P: AsRef<Path>>(&self, path: P, content: &str) -> Result<String, AgentError> {
        let full_path = self.working_dir.join(path.as_ref());
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&full_path, content)?;
        Ok(format!("Successfully wrote {} bytes to {:?}", content.len(), path.as_ref()))
    }

    pub fn run_terminal(&self, cmd: &str) -> String {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", cmd])
                .current_dir(&self.working_dir)
                .output()
        } else {
            Command::new("sh")
                .args(["-c", cmd])
                .current_dir(&self.working_dir)
                .output()
        };

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                format!("Exit code: {}\nSTDOUT:\n{}\nSTDERR:\n{}", out.status.code().unwrap_or(-1), stdout, stderr)
            }
            Err(e) => format!("Command execution failed: {}", e),
        }
    }

    pub async fn execute_task(&self, task_instruction: &str) -> Result<String, AgentError> {
        let prompt = format!(
            "Working Directory: {:?}\n\nTask: {}",
            self.working_dir, task_instruction
        );

        let res = self
            .client
            .complete(&prompt, Some(METACOGNITIVE_ENGINEERING_PROMPT), 0.2, false)
            .await?;

        Ok(res)
    }
}
