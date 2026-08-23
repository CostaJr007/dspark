use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Mock client that returns pre-configured responses and counts calls for validating optimizations
#[derive(Clone)]
pub struct MockModelClient {
    pub responses: Arc<std::sync::Mutex<Vec<MockResponse>>>,
    pub call_count: Arc<AtomicUsize>,
    pub call_history: Arc<std::sync::Mutex<Vec<String>>>,
}

#[derive(Clone, Debug)]
pub struct MockResponse {
    pub content: String,
    pub logprobs: Option<Vec<TokenLogprob>>,
}

#[derive(Clone, Debug)]
pub struct TokenLogprob {
    pub token: String,
    pub logprob: f64,
    pub top_logprobs: Vec<(String, f64)>,
}

impl MockModelClient {
    pub fn new(responses: Vec<MockResponse>) -> Self {
        Self {
            responses: Arc::new(std::sync::Mutex::new(responses)),
            call_count: Arc::new(AtomicUsize::new(0)),
            call_history: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }

    pub async fn complete(&self, prompt: &str, _system: Option<&str>) -> MockResponse {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.call_history.lock().unwrap().push(prompt.to_string());

        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            MockResponse {
                content: r#"{"winner_index": 0, "reasoning": "A is better"}"#.to_string(),
                logprobs: None,
            }
        } else {
            responses.remove(0)
        }
    }

    pub fn reset(&self) {
        self.call_count.store(0, Ordering::SeqCst);
        self.call_history.lock().unwrap().clear();
    }
}
