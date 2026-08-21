//! Multi-provider LLM clients in Rust (DeepSeek, OpenAI, Local Ollama/LM Studio).

use serde::{Deserialize, Serialize};
use std::env;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Missing API key: {0}")]
    MissingApiKey(String),
    #[error("HTTP network request failed: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("JSON serialization/deserialization failed: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("API returned error status {status}: {message}")]
    ApiError { status: u16, message: String },
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct ChatCompletionChoice {
    message: ChatMessageResponse,
}

#[derive(Deserialize)]
struct ChatMessageResponse {
    content: Option<String>,
    reasoning_content: Option<String>,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatCompletionChoice>,
}

pub struct DeepSeekClient {
    api_key: String,
    base_url: String,
    pub model: String,
    http: reqwest::Client,
}

impl DeepSeekClient {
    pub fn new() -> Result<Self, ClientError> {
        let api_key = env::var("DEEPSEEK_API_KEY")
            .map_err(|_| ClientError::MissingApiKey("DEEPSEEK_API_KEY environment variable not set".into()))?;
        
        let base_url = env::var("DEEPSEEK_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepseek.com".to_string());
        
        let model = env::var("DEEPSEEK_MODEL")
            .unwrap_or_else(|_| "deepseek-v4-flash".to_string());

        Ok(Self {
            api_key,
            base_url,
            model,
            http: reqwest::Client::builder()
                .user_agent("DSpark-Rust/0.1.0")
                .build()?,
        })
    }

    pub async fn complete(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
        temperature: f32,
        json_format: bool,
    ) -> Result<String, ClientError> {
        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(ChatMessage {
                role: "system".into(),
                content: sys.into(),
            });
        }
        messages.push(ChatMessage {
            role: "user".into(),
            content: prompt.into(),
        });

        let response_format = if json_format {
            Some(serde_json::json!({ "type": "json_object" }))
        } else {
            None
        };

        let req_body = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            temperature,
            response_format,
        };

        let url = format!("{}/chat/completions", self.base_url);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&req_body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(ClientError::ApiError { status, message: text });
        }

        let body: ChatCompletionResponse = resp.json().await?;
        if let Some(choice) = body.choices.into_iter().next() {
            if let Some(content) = choice.message.content {
                if !content.is_empty() {
                    return Ok(content);
                }
            }
            if let Some(reasoning) = choice.message.reasoning_content {
                return Ok(reasoning);
            }
        }

        Ok(String::new())
    }
}

pub struct OpenAIClient {
    api_key: String,
    base_url: String,
    pub model: String,
    http: reqwest::Client,
}

impl OpenAIClient {
    pub fn new(model: Option<&str>) -> Result<Self, ClientError> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| ClientError::MissingApiKey("OPENAI_API_KEY environment variable not set".into()))?;
        
        let base_url = env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        
        let model = model
            .map(|s| s.to_string())
            .or_else(|| env::var("OPENAI_MODEL").ok())
            .unwrap_or_else(|| "gpt-4o-mini".to_string());

        Ok(Self {
            api_key,
            base_url,
            model,
            http: reqwest::Client::builder()
                .user_agent("DSpark-Rust/0.1.0")
                .build()?,
        })
    }

    pub async fn complete(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
        temperature: f32,
    ) -> Result<String, ClientError> {
        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(ChatMessage {
                role: "system".into(),
                content: sys.into(),
            });
        }
        messages.push(ChatMessage {
            role: "user".into(),
            content: prompt.into(),
        });

        let req_body = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            temperature,
            response_format: None,
        };

        let url = format!("{}/chat/completions", self.base_url);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&req_body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(ClientError::ApiError { status, message: text });
        }

        let body: ChatCompletionResponse = resp.json().await?;
        if let Some(choice) = body.choices.into_iter().next() {
            if let Some(content) = choice.message.content {
                return Ok(content);
            }
        }

        Ok(String::new())
    }
}

pub struct LocalLLMClient {
    base_url: String,
    pub model: String,
    http: reqwest::Client,
}

impl LocalLLMClient {
    pub fn new(base_url: Option<&str>, model: Option<&str>) -> Result<Self, ClientError> {
        let base_url = base_url
            .map(|s| s.to_string())
            .unwrap_or_else(|| "http://localhost:11434/v1".to_string());
        let model = model
            .map(|s| s.to_string())
            .unwrap_or_else(|| "qwen2.5-coder:1.5b".to_string());

        Ok(Self {
            base_url,
            model,
            http: reqwest::Client::builder()
                .user_agent("DSpark-Rust-Local/0.1.0")
                .build()?,
        })
    }

    pub async fn complete(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
        temperature: f32,
    ) -> Result<String, ClientError> {
        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(ChatMessage {
                role: "system".into(),
                content: sys.into(),
            });
        }
        messages.push(ChatMessage {
            role: "user".into(),
            content: prompt.into(),
        });

        let req_body = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            temperature,
            response_format: None,
        };

        let url = format!("{}/chat/completions", self.base_url);
        let resp = self.http.post(&url).json(&req_body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(ClientError::ApiError { status, message: text });
        }

        let body: ChatCompletionResponse = resp.json().await?;
        if let Some(choice) = body.choices.into_iter().next() {
            if let Some(content) = choice.message.content {
                return Ok(content);
            }
        }

        Ok(String::new())
    }
}
