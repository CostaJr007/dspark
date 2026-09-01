//! Multi-provider LLM clients (DeepSeek, OpenAI, Gemini, local Ollama/LM Studio).

use crate::cost::{extract_usage, TokenUsage};
use serde::Serialize;
use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Default)]
pub struct UsageCounters {
    prompt: AtomicU64,
    completion: AtomicU64,
}

impl UsageCounters {
    pub fn add(&self, u: TokenUsage) {
        self.prompt.fetch_add(u.prompt_tokens, Ordering::Relaxed);
        self.completion
            .fetch_add(u.completion_tokens, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> TokenUsage {
        TokenUsage {
            prompt_tokens: self.prompt.load(Ordering::Relaxed),
            completion_tokens: self.completion.load(Ordering::Relaxed),
        }
    }
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

fn http_client(ua: &str, secs: u64) -> Result<reqwest::Client, ClientError> {
    Ok(reqwest::Client::builder()
        .user_agent(ua)
        .timeout(Duration::from_secs(secs))
        .build()?)
}

fn value_as_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(parts) => parts
            .iter()
            .map(|p| {
                if let Some(s) = p.as_str() {
                    return s.to_string();
                }
                p.get("text")
                    .and_then(|t| t.as_str())
                    .or_else(|| p.get("content").and_then(|t| t.as_str()))
                    .unwrap_or("")
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join(""),
        other => other.to_string(),
    }
}

fn message_text(msg: &serde_json::Value) -> String {
    let content = msg
        .get("content")
        .map(value_as_text)
        .unwrap_or_default();
    if !content.trim().is_empty() {
        return content;
    }
    for key in ["reasoning_content", "reasoning", "output_text"] {
        let text = msg.get(key).map(value_as_text).unwrap_or_default();
        if !text.trim().is_empty() {
            return text;
        }
    }
    String::new()
}

/// Parse a chat.completion JSON body or an accidental SSE stream into assistant text.
pub fn parse_chat_completion_text(raw: &str) -> Result<String, ClientError> {
    Ok(parse_chat_completion(raw)?.0)
}

fn parse_chat_completion(raw: &str) -> Result<(String, TokenUsage), ClientError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ClientError::ApiError {
            status: 200,
            message: "empty chat completion body".into(),
        });
    }

    if trimmed.starts_with("data:") || trimmed.contains("\ndata:") {
        let mut content = String::new();
        let mut reasoning = String::new();
        let mut usage = TokenUsage::default();
        for line in raw.lines() {
            let line = line.trim();
            let Some(payload) = line.strip_prefix("data:") else {
                continue;
            };
            let payload = payload.trim();
            if payload.is_empty() || payload == "[DONE]" {
                continue;
            }
            let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) else {
                continue;
            };
            let chunk_usage = extract_usage(&v);
            if chunk_usage.total() > 0 {
                usage = chunk_usage;
            }
            if let Some(delta) = v.pointer("/choices/0/delta") {
                content.push_str(&delta.get("content").map(value_as_text).unwrap_or_default());
                reasoning.push_str(
                    &delta
                        .get("reasoning_content")
                        .map(value_as_text)
                        .unwrap_or_default(),
                );
            }
            if let Some(msg) = v.pointer("/choices/0/message") {
                let t = message_text(msg);
                if !t.trim().is_empty() {
                    content = t;
                }
            }
        }
        let text = if !content.trim().is_empty() {
            content
        } else {
            reasoning
        };
        if text.trim().is_empty() {
            return Err(ClientError::ApiError {
                status: 200,
                message: "SSE chat completion had no content".into(),
            });
        }
        return Ok((text, usage));
    }

    let value: serde_json::Value = serde_json::from_str(trimmed).map_err(|e| {
        ClientError::ApiError {
            status: 200,
            message: format!(
                "chat completion JSON: {e}; body starts: {}",
                trimmed.chars().take(180).collect::<String>().replace('\n', " ")
            ),
        }
    })?;

    let usage = extract_usage(&value);
    if let Some(msg) = value.pointer("/choices/0/message") {
        let text = message_text(msg);
        if !text.trim().is_empty() {
            return Ok((text, usage));
        }
    }
    if let Some(text) = value.get("content").map(value_as_text) {
        if !text.trim().is_empty() {
            return Ok((text, usage));
        }
    }
    Err(ClientError::ApiError {
        status: 200,
        message: "chat completion had empty assistant content".into(),
    })
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
    usage: &UsageCounters,
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
    let text = resp.text().await.unwrap_or_default();
    let (content, tokens) = parse_chat_completion(&text)?;
    usage.add(tokens);
    Ok(content)
}

pub struct DeepSeekClient {
    api_key: String,
    pub base_url: String,
    pub model: String,
    http: reqwest::Client,
    usage: Arc<UsageCounters>,
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
            http: http_client("DSpark/0.1.0", 180)?,
            usage: Arc::new(UsageCounters::default()),
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

        // JSON mode + thinking is unsupported and can yield empty/truncated bodies.
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
            thinking: if json_format {
                Some(serde_json::json!({ "type": "disabled" }))
            } else {
                None
            },
            max_tokens: Some(8192),
        };

        post_chat(
            &self.http,
            &chat_url(&self.base_url),
            Some(&self.api_key),
            &req_body,
            &self.usage,
        )
        .await
    }
}

pub struct OpenAIClient {
    api_key: String,
    pub base_url: String,
    pub model: String,
    http: reqwest::Client,
    usage: Arc<UsageCounters>,
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
            usage: Arc::new(UsageCounters::default()),
        })
    }

    pub fn with_endpoint(
        api_key: String,
        base_url: String,
        model: String,
    ) -> Result<Self, ClientError> {
        Ok(Self {
            api_key,
            base_url,
            model,
            http: http_client("DSpark/0.1.0", 120)?,
            usage: Arc::new(UsageCounters::default()),
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
            stream: Some(false),
            thinking: None,
            max_tokens: Some(8192),
        };

        post_chat(
            &self.http,
            &chat_url(&self.base_url),
            Some(&self.api_key),
            &req_body,
            &self.usage,
        )
        .await
    }
}

pub struct GeminiClient {
    api_key: String,
    pub model: String,
    http: reqwest::Client,
    usage: Arc<UsageCounters>,
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
            usage: Arc::new(UsageCounters::default()),
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
    usage: Arc<UsageCounters>,
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
            usage: Arc::new(UsageCounters::default()),
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
            thinking: None,
            max_tokens: Some(8192),
        };

        post_chat(
            &self.http,
            &chat_url(&self.base_url),
            None,
            &req_body,
            &self.usage,
        )
        .await
    }
}

/// Deterministic scripted LLM client for tests, benches and offline harness runs.
/// Never touches the network: answers are produced by the configured responder.
pub struct ScriptedClient {
    pub model: String,
    responder: Arc<dyn Fn(&str) -> String + Send + Sync>,
    calls: AtomicU64,
}

impl ScriptedClient {
    pub fn new(model: &str, responder: impl Fn(&str) -> String + Send + Sync + 'static) -> Self {
        Self {
            model: model.to_string(),
            responder: Arc::new(responder),
            calls: AtomicU64::new(0),
        }
    }

    /// Always declares candidate A the winner; deterministic tournaments.
    pub fn always_a(model: &str) -> Self {
        Self::new(model, |_| "{\"winner\": \"A\"}".to_string())
    }

    pub fn call_count(&self) -> u64 {
        self.calls.load(Ordering::Relaxed)
    }

    pub async fn complete(
        &self,
        prompt: &str,
        _system_prompt: Option<&str>,
        _temperature: f32,
        _json_format: bool,
    ) -> Result<String, ClientError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok((self.responder)(prompt))
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn header_value(headers: &[u8], name: &str) -> Option<usize> {
    let text = std::str::from_utf8(headers).ok()?;
    for line in text.split("\r\n") {
        let mut parts = line.splitn(2, ':');
        if parts.next()?.trim().eq_ignore_ascii_case(name) {
            return parts.next()?.trim().parse().ok();
        }
    }
    None
}

/// Spawns a minimal OpenAI-compatible chat-completions server on an ephemeral
/// localhost port and returns its `/v1` base URL. Each response body is produced
/// by `responder(request_body_json)`. Useful for offline end-to-end engine tests
/// and benchmarks that must exercise the real HTTP path of the pipeline.
pub fn spawn_mock_chat_server(responder: Arc<dyn Fn(&str) -> String + Send + Sync>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock chat server");
    let addr = listener.local_addr().expect("mock chat server addr");
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            let mut buf: Vec<u8> = Vec::new();
            let mut chunk = [0u8; 4096];
            const HEADER_END: &[u8] = b"\r\n\r\n";
            let mut body_start = None;
            loop {
                match stream.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        buf.extend_from_slice(&chunk[..n]);
                        if let Some(pos) = find_subsequence(&buf, HEADER_END) {
                            body_start = Some(pos + HEADER_END.len());
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            let payload = match body_start {
                Some(start) => {
                    let want = header_value(&buf[..start - HEADER_END.len()], "Content-Length");
                    while let Some(len) = want {
                        if buf.len() >= start + len {
                            break;
                        }
                        match stream.read(&mut chunk) {
                            Ok(0) | Err(_) => break,
                            Ok(n) => buf.extend_from_slice(&chunk[..n]),
                        }
                    }
                    String::from_utf8_lossy(&buf[start..]).to_string()
                }
                None => String::new(),
            };
            let response_payload = responder(&payload);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_payload.len(),
                response_payload
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    format!("http://{}/v1", addr)
}

/// Unified client selected by a model identifier (`gpt-4o-mini`, `local:qwen2.5-coder:1.5b`, ...).
pub enum ModelClient {
    DeepSeek(DeepSeekClient),
    OpenAI(OpenAIClient),
    Gemini(GeminiClient),
    Local(LocalLLMClient),
    Scripted(ScriptedClient),
}

impl ModelClient {
    pub fn from_spec(spec: &str) -> Result<Self, ClientError> {
        let spec = spec.trim();
        let lower = spec.to_lowercase();

        if lower == "mock" || lower.starts_with("mock:") {
            let model_name = spec.split_once(':').map(|(_, m)| m).unwrap_or("mock");
            return Ok(Self::Scripted(ScriptedClient::always_a(model_name)));
        }
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
        } else if lower.starts_with("groq:") {
            let model_name = spec.split_once(':').map(|(_, m)| m).unwrap_or("qwen/qwen3.8-27b");
            let api_key = env::var("GROQ_API_KEY")
                .or_else(|_| env::var("OPENAI_API_KEY"))
                .map_err(|_| ClientError::MissingApiKey("GROQ_API_KEY environment variable not set".into()))?;
            let base_url = env::var("GROQ_BASE_URL")
                .unwrap_or_else(|_| "https://api.groq.com/openai/v1".to_string());
            Ok(Self::OpenAI(OpenAIClient::with_endpoint(
                api_key,
                base_url,
                model_name.to_string(),
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
            Self::Scripted(c) => &c.model,
        }
    }

    pub fn usage(&self) -> TokenUsage {
        match self {
            Self::DeepSeek(c) => c.usage.snapshot(),
            Self::OpenAI(c) => c.usage.snapshot(),
            Self::Gemini(c) => c.usage.snapshot(),
            Self::Local(c) => c.usage.snapshot(),
            Self::Scripted(_) => TokenUsage::default(),
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
            Self::Scripted(c) => {
                c.complete(prompt, system_prompt, temperature, json_format)
                    .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_chat_completion_text;

    #[test]
    fn parses_string_content() {
        let raw = r#"{"choices":[{"message":{"content":"hello","role":"assistant"}}]}"#;
        assert_eq!(parse_chat_completion_text(raw).unwrap(), "hello");
    }

    #[test]
    fn parses_array_content() {
        let raw = r#"{"choices":[{"message":{"content":[{"type":"text","text":"ab"},{"type":"text","text":"c"}]}}]}"#;
        assert_eq!(parse_chat_completion_text(raw).unwrap(), "abc");
    }

    #[test]
    fn falls_back_to_reasoning_content() {
        let raw = r#"{"choices":[{"message":{"content":null,"reasoning_content":"think"}}]}"#;
        assert_eq!(parse_chat_completion_text(raw).unwrap(), "think");
    }

    #[test]
    fn parses_usage_tokens() {
        let raw = r#"{"choices":[{"message":{"content":"ok"}}],"usage":{"prompt_tokens":11,"completion_tokens":7}}"#;
        let (_, usage) = super::parse_chat_completion(raw).unwrap();
        assert_eq!(usage.prompt_tokens, 11);
        assert_eq!(usage.completion_tokens, 7);
    }

    #[test]
    fn parses_sse_stream() {
        let raw = "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\
data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\
data: [DONE]\n";
        assert_eq!(parse_chat_completion_text(raw).unwrap(), "Hello");
    }

    #[tokio::test]
    async fn scripted_client_counts_calls_and_answers() {
        let c = super::ModelClient::from_spec("mock:judge-x").unwrap();
        assert_eq!(c.model_name(), "judge-x");
        let out = c.complete("p", None, 0.0, false).await.unwrap();
        assert!(out.contains("\"winner\": \"A\""));
        assert_eq!(c.usage().total(), 0);
    }

    #[test]
    fn spawns_mock_chat_server() {
        let url = super::spawn_mock_chat_server(std::sync::Arc::new(|_| {
            "{\"winner\": \"A\"}".to_string()
        }));
        assert!(url.starts_with("http://127.0.0.1:"));
    }
}
