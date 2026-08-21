//! High-throughput draft generator (Gemini / any ModelClient).

use crate::client::{ClientError, ModelClient};
use crate::prompts::DRAFT_SYSTEM_INSTRUCTION;
use crate::util::extract_code_blocks;

pub struct GeminiGenerator {
    client: ModelClient,
}

impl GeminiGenerator {
    pub fn new() -> Result<Self, ClientError> {
        Ok(Self {
            client: ModelClient::from_spec(
                &std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.5-flash".into()),
            )?,
        })
    }

    pub fn with_spec(spec: &str) -> Result<Self, ClientError> {
        Ok(Self {
            client: ModelClient::from_spec(spec)?,
        })
    }

    pub fn with_client(client: ModelClient) -> Self {
        Self { client }
    }

    pub async fn generate_draft(
        &self,
        prompt: &str,
        language: Option<&str>,
        temperature: f32,
    ) -> Result<String, ClientError> {
        let lang_directive = language
            .map(|l| format!(" (Target language: {})", l))
            .unwrap_or_default();
        let full_prompt = format!(
            "Implement the following feature or function{lang_directive}:\n\n{prompt}"
        );
        let raw = self
            .client
            .complete(
                &full_prompt,
                Some(DRAFT_SYSTEM_INSTRUCTION),
                temperature,
                false,
            )
            .await?;
        Ok(extract_code_blocks(&raw))
    }
}
