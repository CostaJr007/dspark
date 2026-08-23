//! Speculative Drafter module.
//! Generates N trajectories in parallel and applies the Sequential Dependency Injection module.

use crate::client::{ClientError, ModelClient};
use crate::utils::ast_resolver::{create_resolver, CodeBlock, DependencyResolver};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DraftTrajectory {
    pub id: usize,
    pub full_code: String,
    pub code_blocks: Vec<CodeBlock>,
    pub confidence_score: f64,
    pub ast_valid: bool,
}

pub struct SpeculativeDrafter {
    client: Arc<ModelClient>,
    n_trajectories: usize,
    semaphore: Arc<Semaphore>,
    resolver: Box<dyn DependencyResolver>,
}

impl SpeculativeDrafter {
    pub fn new(client: ModelClient, n_trajectories: usize) -> Self {
        Self {
            client: Arc::new(client),
            n_trajectories,
            semaphore: Arc::new(Semaphore::new(n_trajectories.max(1))),
            resolver: create_resolver(),
        }
    }

    pub fn with_resolver(client: ModelClient, n_trajectories: usize, resolver: Box<dyn DependencyResolver>) -> Self {
        Self {
            client: Arc::new(client),
            n_trajectories,
            semaphore: Arc::new(Semaphore::new(n_trajectories.max(1))),
            resolver,
        }
    }

    pub fn with_model(model_name: &str, n_trajectories: usize) -> Result<Self, ClientError> {
        let client = ModelClient::from_spec(model_name)?;
        Ok(Self::new(client, n_trajectories))
    }

    /// Generates N draft trajectories in parallel
    pub async fn generate_trajectories(&self, prompt: &str) -> Vec<DraftTrajectory> {
        let mut handles = Vec::new();

        for i in 0..self.n_trajectories {
            let client = Arc::clone(&self.client);
            let sem = Arc::clone(&self.semaphore);
            let prompt_text = prompt.to_string();
            let temperature = 0.2 + (i as f32 * 0.15); // Diversity in drafting

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.ok();
                let res = client.complete(&prompt_text, None, temperature, false).await;
                (i, res)
            }));
        }

        let mut trajectories = Vec::new();

        for handle in handles {
            if let Ok((id, Ok(raw_code))) = handle.await {
                let blocks = self.resolver.split_into_blocks(&raw_code);
                let (dep_graph, ast_valid) = self.resolver.resolve(&blocks, "rust");
                let ordered_blocks = dep_graph.topological_sort();
                let full_code = if ordered_blocks.is_empty() {
                    raw_code
                } else {
                    ordered_blocks.iter().map(|b| b.code.as_str()).collect::<Vec<_>>().join("\n\n")
                };

                trajectories.push(DraftTrajectory {
                    id,
                    full_code,
                    code_blocks: ordered_blocks,
                    confidence_score: 0.0,
                    ast_valid,
                });
            }
        }

        trajectories
    }

    /// Sequential Module: Filter invalid AST drafts and order topographically
    pub fn apply_sequential_module(&self, trajectories: Vec<DraftTrajectory>) -> Vec<DraftTrajectory> {
        trajectories
            .into_iter()
            .filter(|t| t.ast_valid || !t.code_blocks.is_empty())
            .collect()
    }
}
