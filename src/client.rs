//! Multi-provider LLM clients (DeepSeek, OpenAI, Gemini, local Ollama/LM Studio).

use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::time::timeout;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
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

fn http_client(ua: &str, secs: u64) -> Result<reqwest::Client, ClientError> {
    Ok(reqwest::Client::builder()
        .user_agent(ua)
        .timeout(Duration::from_secs(secs))
        .build()?)
}

fn first_choice_text(body: ChatCompletionResponse) -> String {
    if let Some(choice) = body.choices.into_iter().next() {
        if let Some(content) = choice.message.content {
            if !content.is_empty() {
                return content;
            }
        }
        if let Some(reasoning) = choice.message.reasoning_content {
            return reasoning;
        }
    }
    String::new()
}

fn chat_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    if base.ends_with("/chat/completions") {
        base.to_string()
    } else {
        format!("{}/chat/completions", base)
    }
}

async fn post_chat(
    http: &reqwest::Client,
    url: &str,
    api_key: Option<&str>,
    body: &ChatCompletionRequest,
) -> Result<String, ClientError> {
    let mut req = http.post(url).json(body);
    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {}", key));
    }
    let resp = req.send().await?;
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(ClientError::ApiError {
            status,
            message: text,
        });
    }
    let body: ChatCompletionResponse = resp.json().await?;
    Ok(first_choice_text(body))
}

pub struct DeepSeekClient {
    api_key: String,
    pub base_url: String,
    pub model: String,
    http: reqwest::Client,
}

impl DeepSeekClient {
    pub fn new() -> Result<Self, ClientError> {
        Self::with_model(None)
    }

    pub fn with_model(model: Option<&str>) -> Result<Self, ClientError> {
        let api_key = env::var("DEEPSEEK_API_KEY").map_err(|_| {
            ClientError::MissingApiKey("DEEPSEEK_API_KEY environment variable not set".into())
        })?;

        let base_url =
            env::var("DEEPSEEK_BASE_URL").unwrap_or_else(|_| "https://api.deepseek.com".to_string());

        let model = model
            .map(|s| s.to_string())
            .or_else(|| env::var("DEEPSEEK_MODEL").ok())
            .unwrap_or_else(|| "deepseek-v4-flash".to_string());

        Ok(Self {
            api_key,
            base_url,
            model,
            http: http_client("DSpark/0.1.0", 120)?,
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

        let req_body = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            temperature,
            response_format: if json_format {
                Some(serde_json::json!({ "type": "json_object" }))
            } else {
                None
            },
            stream: None,
        };

        post_chat(
            &self.http,
            &chat_url(&self.base_url),
            Some(&self.api_key),
            &req_body,
        )
        .await
    }
}

pub struct OpenAIClient {
    api_key: String,
    pub base_url: String,
    pub model: String,
    http: reqwest::Client,
}

impl OpenAIClient {
    pub fn new(model: Option<&str>) -> Result<Self, ClientError> {
        let api_key = env::var("OPENAI_API_KEY").map_err(|_| {
            ClientError::MissingApiKey("OPENAI_API_KEY environment variable not set".into())
        })?;

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
            http: http_client("DSpark/0.1.0", 120)?,
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

        let req_body = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            temperature,
            response_format: if json_format {
                Some(serde_json::json!({ "type": "json_object" }))
            } else {
                None
            },
            stream: None,
        };

        post_chat(
            &self.http,
            &chat_url(&self.base_url),
            Some(&self.api_key),
            &req_body,
        )
        .await
    }
}

pub struct GeminiClient {
    api_key: String,
    pub model: String,
    http: reqwest::Client,
}

impl GeminiClient {
    pub fn new(model: Option<&str>) -> Result<Self, ClientError> {
        let api_key = env::var("GEMINI_API_KEY").map_err(|_| {
            ClientError::MissingApiKey("GEMINI_API_KEY environment variable not set".into())
        })?;
        let model = model
            .map(|s| s.to_string())
            .or_else(|| env::var("GEMINI_MODEL").ok())
            .unwrap_or_else(|| "gemini-2.5-flash".to_string());

        Ok(Self {
            api_key,
            model,
            http: http_client("DSpark/0.1.0", 120)?,
        })
    }

    pub async fn complete(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
        temperature: f32,
    ) -> Result<String, ClientError> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let mut contents = Vec::new();
        if let Some(sys) = system_prompt {
            contents.push(serde_json::json!({
                "role": "user",
                "parts": [{ "text": format!("System Directive:\n{}", sys) }]
            }));
            contents.push(serde_json::json!({
                "role": "model",
                "parts": [{ "text": "Understood. I will adhere strictly to these directives." }]
            }));
        }
        contents.push(serde_json::json!({
            "role": "user",
            "parts": [{ "text": prompt }]
        }));

        let payload = serde_json::json!({
            "contents": contents,
            "generationConfig": { "temperature": temperature }
        });

        let resp = self.http.post(&url).json(&payload).send().await?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(ClientError::ApiError {
                status,
                message: text,
            });
        }

        let data: serde_json::Value = resp.json().await?;
        let mut out = String::new();
        if let Some(parts) = data
            .pointer("/candidates/0/content/parts")
            .and_then(|v| v.as_array())
        {
            for part in parts {
                if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                    out.push_str(text);
                }
            }
        }
        Ok(out)
    }
}

#[derive(Debug, Clone)]
pub struct LocalEndpoint {
    pub name: String,
    pub v1_url: String,
}

pub struct LocalLLMClient {
    pub base_url: String,
    pub model: String,
    http: reqwest::Client,
}

impl LocalLLMClient {
    pub fn new(base_url: Option<&str>, model: Option<&str>) -> Result<Self, ClientError> {
        let base_url = base_url
            .map(|s| s.to_string())
            .or_else(|| env::var("LOCAL_LLM_URL").ok())
            .or_else(|| env::var("OLLAMA_BASE_URL").ok())
            .unwrap_or_else(|| "http://localhost:11434/v1".to_string());
        let model = model
            .map(|s| s.to_string())
            .or_else(|| env::var("LOCAL_LLM_MODEL").ok())
            .unwrap_or_else(|| "qwen2.5-coder:1.5b".to_string());

        Ok(Self {
            base_url,
            model,
            http: http_client("DSpark-Local/0.1.0", 180)?,
        })
    }

    pub async fn detect_active_endpoints() -> Vec<LocalEndpoint> {
        let candidates = [
            ("Ollama", 11434_u16, "http://localhost:11434/v1"),
            ("LM Studio", 1234, "http://localhost:1234/v1"),
            ("vLLM / LocalAI", 8000, "http://localhost:8000/v1"),
        ];
        let mut active = Vec::new();
        for (name, port, url) in candidates {
            let probe = timeout(
                Duration::from_millis(80),
                TcpStream::connect(("127.0.0.1", port)),
            )
            .await;
            if matches!(probe, Ok(Ok(_))) {
                active.push(LocalEndpoint {
                    name: name.to_string(),
                    v1_url: url.to_string(),
                });
            }
        }
        active
    }

    pub async fn list_models(&self) -> Vec<String> {
        let models_url = format!("{}/models", self.base_url.trim_end_matches('/'));
        if let Ok(resp) = self
            .http
            .get(&models_url)
            .timeout(Duration::from_secs(3))
            .send()
            .await
        {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(arr) = data.get("data").and_then(|d| d.as_array()) {
                    return arr
                        .iter()
                        .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(|s| s.to_string()))
                        .collect();
                }
            }
        }

        let root = self.base_url.trim_end_matches('/').trim_end_matches("/v1");
        let tags_url = format!("{}/api/tags", root);
        if let Ok(resp) = self
            .http
            .get(&tags_url)
            .timeout(Duration::from_secs(3))
            .send()
            .await
        {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(arr) = data.get("models").and_then(|d| d.as_array()) {
                    return arr
                        .iter()
                        .filter_map(|m| {
                            m.get("name")
                                .and_then(|id| id.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect();
                }
            }
        }
        Vec::new()
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

        let req_body = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            temperature,
            response_format: if json_format {
                Some(serde_json::json!({ "type": "json_object" }))
            } else {
                None
            },
            stream: Some(false),
        };

        post_chat(&self.http, &chat_url(&self.base_url), None, &req_body).await
    }
}

/// Unified client selected by a model identifier (`gpt-4o-mini`, `local:qwen2.5-coder:1.5b`, ...).
pub enum ModelClient {
    DeepSeek(DeepSeekClient),
    OpenAI(OpenAIClient),
    Gemini(GeminiClient),
    Local(LocalLLMClient),
}

impl ModelClient {
    pub fn from_spec(spec: &str) -> Result<Self, ClientError> {
        let spec = spec.trim();
        let lower = spec.to_lowercase();

        if lower.starts_with("local:")
            || lower.starts_with("ollama:")
            || lower.starts_with("lmstudio:")
        {
            let model_name = spec.split_once(':').map(|(_, m)| m).unwrap_or(spec);
            let base_url = if lower.starts_with("lmstudio:") {
                "http://localhost:1234/v1"
            } else {
                "http://localhost:11434/v1"
            };
            Ok(Self::Local(LocalLLMClient::new(
                Some(base_url),
                Some(model_name),
            )?))
        } else if lower.starts_with("openai:") || lower.contains("gpt-") {
            let model_name = spec.split_once(':').map(|(_, m)| m).unwrap_or(spec);
            Ok(Self::OpenAI(OpenAIClient::new(Some(model_name))?))
        } else if lower.starts_with("gemini:") || lower.contains("gemini") {
            let model_name = spec.split_once(':').map(|(_, m)| m).unwrap_or(spec);
            Ok(Self::Gemini(GeminiClient::new(Some(model_name))?))
        } else {
            let model_name = spec.split_once(':').map(|(_, m)| m).unwrap_or(spec);
            Ok(Self::DeepSeek(DeepSeekClient::with_model(Some(model_name))?))
        }
    }

    pub fn model_name(&self) -> &str {
        match self {
            Self::DeepSeek(c) => &c.model,
            Self::OpenAI(c) => &c.model,
            Self::Gemini(c) => &c.model,
            Self::Local(c) => &c.model,
        }
    }

    pub async fn complete(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
        temperature: f32,
        json_format: bool,
    ) -> Result<String, ClientError> {
        match self {
            Self::DeepSeek(c) => {
                c.complete(prompt, system_prompt, temperature, json_format)
                    .await
            }
            Self::OpenAI(c) => {
                c.complete(prompt, system_prompt, temperature, json_format)
                    .await
            }
            Self::Gemini(c) => c.complete(prompt, system_prompt, temperature).await,
            Self::Local(c) => {
                match c
                    .complete(prompt, system_prompt, temperature, json_format)
                    .await
                {
                    Ok(s) => Ok(s),
                    Err(_) if json_format => {
                        c.complete(prompt, system_prompt, temperature, false).await
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }
}
