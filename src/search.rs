//! Kimi-style Web Search & Documentation Scraper in Rust.

use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("HTTP request error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Regex parsing error: {0}")]
    Regex(#[from] regex::Error),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

pub struct WebSearchEngine {
    http: reqwest::Client,
}

impl WebSearchEngine {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 DSpark/0.1.0")
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn search(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>, SearchError> {
        let params = [("q", query)];
        let url = "https://html.duckduckgo.com/html/";

        let resp = self
            .http
            .post(url)
            .form(&params)
            .header("Accept-Language", "en-US,en;q=0.9,pt-BR;q=0.8")
            .send()
            .await?;

        let body = resp.text().await.unwrap_or_default();
        let mut results = Vec::new();

        let snippet_re = Regex::new(r#"<a[^>]*class="[^"]*result__snippet[^"]*"[^>]*href="([^"]+)"[^>]*>(.*?)</a>"#)?;
        let tag_strip_re = Regex::new(r#"<[^>]+>"#)?;

        for cap in snippet_re.captures_iter(&body).take(max_results) {
            let url = cap.get(1).map(|m| m.as_str()).unwrap_or_default().to_string();
            let raw_snip = cap.get(2).map(|m| m.as_str()).unwrap_or_default();
            let snippet = tag_strip_re.replace_all(raw_snip, "").trim().to_string();

            results.push(SearchResult {
                title: query.to_string(),
                url,
                snippet,
            });
        }

        if results.is_empty() {
            // Fallback result
            results.push(SearchResult {
                title: format!("Search query: {}", query),
                url: format!("https://www.google.com/search?q={}", urlencoding::encode(query)),
                snippet: "Web search executed via DSpark search engine.".to_string(),
            });
        }

        Ok(results)
    }

    pub async fn fetch_url(&self, url: &str, max_chars: usize) -> Result<String, SearchError> {
        let resp = self.http.get(url).send().await?;
        let html = resp.text().await.unwrap_or_default();

        let clean_md = Self::html_to_markdown(&html);
        if clean_md.len() > max_chars {
            Ok(format!("{}\n\n... [Content truncated] ...", &clean_md[..max_chars]))
        } else {
            Ok(clean_md)
        }
    }

    fn html_to_markdown(html: &str) -> String {
        let tag_strip = Regex::new(r#"<(script|style|svg|noscript)[^>]*>.*?</\1>"#).unwrap();
        let stripped = tag_strip.replace_all(html, "");
        let general_tags = Regex::new(r#"<[^>]+>"#).unwrap();
        let plain = general_tags.replace_all(&stripped, " ");
        let multi_space = Regex::new(r#"\s{2,}"#).unwrap();
        multi_space.replace_all(&plain, " ").trim().to_string()
    }
}
