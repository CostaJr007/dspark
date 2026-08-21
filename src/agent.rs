//! Autonomous coding agent runtime.

use crate::client::{ClientError, ModelClient};
use crate::curator::{CuratorError, DeepSeekCurator};
use crate::prompts::METACOGNITIVE_ENGINEERING_PROMPT;
use crate::search::{SearchError, WebSearchEngine};
use crate::tools::{walk_files, ToolRegistry};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Client error: {0}")]
    Client(#[from] ClientError),
    #[error("Search error: {0}")]
    Search(#[from] SearchError),
    #[error("Curator error: {0}")]
    Curator(#[from] CuratorError),
    #[error("{0}")]
    Other(String),
}

pub struct DSparkAgent {
    pub working_dir: PathBuf,
    pub generator_spec: String,
    pub curator_spec: String,
    client: Option<ModelClient>,
    curator: Option<DeepSeekCurator>,
    pub search_engine: WebSearchEngine,
    tools: ToolRegistry,
}

impl DSparkAgent {
    pub fn new(working_dir: Option<PathBuf>) -> Result<Self, AgentError> {
        Self::with_models(working_dir, "deepseek-v4-flash", "deepseek-v4-flash")
    }

    pub fn with_models(
        working_dir: Option<PathBuf>,
        generator: &str,
        curator: &str,
    ) -> Result<Self, AgentError> {
        let wd = working_dir.unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        });
        let client = ModelClient::from_spec(generator).ok();
        let curator_engine = DeepSeekCurator::with_model(curator).ok();
        let tools = ToolRegistry::new(
            wd.clone(),
            DeepSeekCurator::with_model(curator).ok(),
        );
        Ok(Self {
            working_dir: wd,
            generator_spec: generator.to_string(),
            curator_spec: curator.to_string(),
            client,
            curator: curator_engine,
            search_engine: WebSearchEngine::new(),
            tools,
        })
    }

    pub fn set_models(&mut self, generator: &str, curator: &str) -> Result<(), AgentError> {
        self.generator_spec = generator.to_string();
        self.curator_spec = curator.to_string();
        self.client = Some(ModelClient::from_spec(generator)?);
        self.curator = Some(DeepSeekCurator::with_model(curator)?);
        self.tools = ToolRegistry::new(
            self.working_dir.clone(),
            DeepSeekCurator::with_model(curator).ok(),
        );
        Ok(())
    }

    pub fn read_file<P: AsRef<Path>>(&self, path: P) -> Result<String, AgentError> {
        let res = self
            .tools
            .read_file(&path.as_ref().to_string_lossy(), None, None);
        if res.success {
            Ok(res.output)
        } else {
            Err(AgentError::Other(res.error.unwrap_or_default()))
        }
    }

    pub fn write_file<P: AsRef<Path>>(&self, path: P, content: &str) -> Result<String, AgentError> {
        let res = self
            .tools
            .write_file(&path.as_ref().to_string_lossy(), content);
        if res.success {
            Ok(res.output)
        } else {
            Err(AgentError::Other(res.error.unwrap_or_default()))
        }
    }

    pub fn list_files(&self, relative_path: &str) -> Vec<String> {
        let target = if Path::new(relative_path).is_absolute() {
            PathBuf::from(relative_path)
        } else {
            self.working_dir.join(relative_path)
        };
        if !target.exists() {
            return Vec::new();
        }
        let mut entries = Vec::new();
        walk_files(&target, &self.working_dir, &mut entries, 100);
        entries.sort();
        entries
    }

    pub fn run_terminal(&self, cmd: &str) -> String {
        let res = self.tools.run_terminal(cmd, 60);
        if res.success {
            res.output
        } else {
            res.error.unwrap_or(res.output)
        }
    }

    pub async fn search_web(&self, query: &str, max_results: usize) -> Result<String, AgentError> {
        let results = self.search_engine.search(query, max_results).await?;
        if results.is_empty() {
            return Ok(format!("No web search results found for: {}", query));
        }
        let mut output = vec![format!("Web search results for '{}':\n", query)];
        for (idx, res) in results.iter().enumerate() {
            output.push(format!(
                "{}. {}\n   URL: {}\n   {}\n",
                idx + 1,
                res.title,
                res.url,
                res.snippet
            ));
        }
        Ok(output.join("\n"))
    }

    pub async fn fetch_url(&self, url: &str) -> Result<String, AgentError> {
        Ok(self.search_engine.fetch_url(url, 8000).await?)
    }

    pub fn curator(&self) -> Result<&DeepSeekCurator, AgentError> {
        self.curator
            .as_ref()
            .ok_or_else(|| AgentError::Other(
                "Curator is not configured. Set DEEPSEEK_API_KEY (or pick a local model with /models).".into(),
            ))
    }

    pub async fn execute_task(&self, task_instruction: &str) -> Result<String, AgentError> {
        let client = self.client.as_ref().ok_or_else(|| {
            AgentError::Other(
                "No LLM client configured. Set DEEPSEEK_API_KEY / OPENAI_API_KEY / GEMINI_API_KEY, or use /models to pick a local engine.".into(),
            )
        })?;
        let prompt = format!(
            "Working Directory: {:?}\n\nTask: {}",
            self.working_dir, task_instruction
        );
        Ok(client
            .complete(&prompt, Some(METACOGNITIVE_ENGINEERING_PROMPT), 0.2, false)
            .await?)
    }
}
