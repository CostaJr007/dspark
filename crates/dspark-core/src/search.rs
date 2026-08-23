//! Live web search and documentation scraper (search + fetch).

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::OnceLock;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("HTTP request error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("{0}")]
    Other(String),
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

static TAG_STRIP: OnceLock<Regex> = OnceLock::new();
static SCRIPT_STRIPS: OnceLock<Vec<Regex>> = OnceLock::new();
static RESULT_A: OnceLock<Regex> = OnceLock::new();
static RESULT_SNIP: OnceLock<Regex> = OnceLock::new();
static HEADER_RES: OnceLock<Vec<(Regex, String)>> = OnceLock::new();
static PRE_CODE: OnceLock<Regex> = OnceLock::new();
static INLINE_CODE: OnceLock<Regex> = OnceLock::new();
static LI_RE: OnceLock<Regex> = OnceLock::new();
static P_RE: OnceLock<Regex> = OnceLock::new();
static BR_RE: OnceLock<Regex> = OnceLock::new();
static MULTI_NL: OnceLock<Regex> = OnceLock::new();

fn tag_strip() -> &'static Regex {
    TAG_STRIP.get_or_init(|| Regex::new(r"<[^>]+>").expect("tag strip"))
}

impl Default for WebSearchEngine {
    fn default() -> Self {
        Self::new()
    }
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

    pub fn provider_name() -> &'static str {
        if env::var("TAVILY_API_KEY").is_ok() {
            "Tavily"
        } else {
            "DuckDuckGo HTML"
        }
    }

    pub async fn search(
        &self,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        if env::var("TAVILY_API_KEY").is_ok() {
            let tavily = self.search_tavily(query, max_results).await?;
            if !tavily.is_empty() {
                return Ok(tavily);
            }
        }
        self.search_duckduckgo(query, max_results).await
    }

    async fn search_tavily(
        &self,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let api_key = env::var("TAVILY_API_KEY").unwrap_or_default();
        let payload = serde_json::json!({
            "api_key": api_key,
            "query": query,
            "max_results": max_results,
            "include_answer": false,
            "search_depth": "advanced",
        });
        let resp = self
            .http
            .post("https://api.tavily.com/search")
            .json(&payload)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(SearchError::Other(format!(
                "Tavily error {status}: {body}"
            )));
        }
        let data: serde_json::Value = resp.json().await?;
        let mut results = Vec::new();
        if let Some(arr) = data.get("results").and_then(|v| v.as_array()) {
            for item in arr.iter().take(max_results) {
                results.push(SearchResult {
                    title: item
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or(query)
                        .to_string(),
                    url: item
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    snippet: item
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                });
            }
        }
        Ok(results)
    }

    async fn search_duckduckgo(
        &self,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let params = [("q", query)];
        let resp = self
            .http
            .post("https://html.duckduckgo.com/html/")
            .form(&params)
            .header("Accept-Language", "en-US,en;q=0.9,pt-BR;q=0.8")
            .send()
            .await?;
        let body = resp.text().await.unwrap_or_default();
        Ok(parse_duckduckgo(&body, query, max_results))
    }

    pub async fn fetch_url(&self, url: &str, max_chars: usize) -> Result<String, SearchError> {
        let resp = self.http.get(url).send().await?;
        let html = resp.text().await.unwrap_or_default();
        let clean_md = html_to_markdown(&html);
        if clean_md.chars().count() > max_chars {
            let truncated: String = clean_md.chars().take(max_chars).collect();
            Ok(format!(
                "{}\n\n... [Content truncated for context window] ...",
                truncated
            ))
        } else {
            Ok(clean_md)
        }
    }

    pub async fn research_topic(&self, topic: &str, max_sources: usize) -> String {
        let search_results = match self.search(topic, max_sources).await {
            Ok(r) if !r.is_empty() => r,
            _ => return format!("No results found for topic: {}", topic),
        };

        let mut report = vec![format!("## Web Research Results for: '{}'\n", topic)];
        for (idx, res) in search_results.iter().enumerate() {
            report.push(format!("### Source {}: [{}]({})", idx + 1, res.title, res.url));
            report.push(format!("**Snippet**: {}\n", res.snippet));
            let page_text = self
                .fetch_url(&res.url, 1500)
                .await
                .unwrap_or_else(|e| format!("Failed to fetch: {}", e));
            let excerpt: String = page_text.chars().take(800).collect();
            report.push(format!("**Page Excerpt**:\n```\n{}\n```\n", excerpt));
        }
        report.join("\n")
    }
}

fn extract_uddg(href: &str) -> String {
    if let Some((_, query)) = href.split_once('?') {
        for part in query.split('&') {
            if let Some(v) = part.strip_prefix("uddg=") {
                return urlencoding::decode(v)
                    .map(|s| s.into_owned())
                    .unwrap_or_else(|_| v.to_string());
            }
        }
    }
    href.to_string()
}

fn decode_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

fn parse_duckduckgo(body: &str, query: &str, max_results: usize) -> Vec<SearchResult> {
    let result_a = RESULT_A.get_or_init(|| {
        Regex::new(r#"(?s)<a[^>]*class="[^"]*result__a[^"]*"[^>]*href="([^"]+)"[^>]*>(.*?)</a>"#)
            .expect("result_a")
    });
    let result_snip = RESULT_SNIP.get_or_init(|| {
        Regex::new(r#"(?s)<a[^>]*class="[^"]*result__snippet[^"]*"[^>]*>(.*?)</a>"#)
            .expect("result_snip")
    });

    let titles: Vec<(String, String)> = result_a
        .captures_iter(body)
        .map(|c| {
            (
                c.get(1).map(|m| m.as_str()).unwrap_or_default().to_string(),
                c.get(2).map(|m| m.as_str()).unwrap_or_default().to_string(),
            )
        })
        .collect();
    let snippets: Vec<String> = result_snip
        .captures_iter(body)
        .map(|c| c.get(1).map(|m| m.as_str()).unwrap_or_default().to_string())
        .collect();

    let mut results = Vec::new();
    let n = titles.len().min(max_results);
    for (i, (raw_href, raw_title)) in titles.iter().take(n).enumerate() {
        let raw_snippet = snippets.get(i).map(String::as_str).unwrap_or("");
        let clean_title = decode_entities(tag_strip().replace_all(raw_title, "").trim());
        let clean_snippet = decode_entities(tag_strip().replace_all(raw_snippet, "").trim());
        results.push(SearchResult {
            title: if clean_title.is_empty() {
                query.to_string()
            } else {
                clean_title
            },
            url: extract_uddg(raw_href),
            snippet: clean_snippet,
        });
    }
    results
}

pub fn html_to_markdown(html: &str) -> String {
    let script_strips = SCRIPT_STRIPS.get_or_init(|| {
        ["script", "style", "svg", "noscript"]
            .iter()
            .map(|tag| {
                Regex::new(&format!(r"(?is)<{tag}[^>]*>.*?</{tag}>")).expect("script strip")
            })
            .collect()
    });
    let mut text = html.to_string();
    for re in script_strips.iter() {
        text = re.replace_all(&text, "").to_string();
    }

    let headers = HEADER_RES.get_or_init(|| {
        (1..=6)
            .rev()
            .map(|i| {
                (
                    Regex::new(&format!(r"(?is)<h{i}[^>]*>(.*?)</h{i}>")).expect("header"),
                    "#".repeat(i),
                )
            })
            .collect()
    });
    for (re, hashes) in headers.iter() {
        text = re
            .replace_all(&text, format!("\n{} $1\n", hashes))
            .to_string();
    }

    let pre_code = PRE_CODE.get_or_init(|| {
        Regex::new(r"(?is)<pre[^>]*><code[^>]*>(.*?)</code></pre>").expect("pre code")
    });
    text = pre_code.replace_all(&text, "\n```\n$1\n```\n").to_string();

    let inline_code =
        INLINE_CODE.get_or_init(|| Regex::new(r"(?is)<code[^>]*>(.*?)</code>").expect("code"));
    text = inline_code.replace_all(&text, "`$1`").to_string();

    let li_re = LI_RE.get_or_init(|| Regex::new(r"(?is)<li[^>]*>(.*?)</li>").expect("li"));
    text = li_re.replace_all(&text, "\n* $1").to_string();

    let p_re = P_RE.get_or_init(|| Regex::new(r"(?is)<p[^>]*>(.*?)</p>").expect("p"));
    text = p_re.replace_all(&text, "\n$1\n").to_string();

    let br_re = BR_RE.get_or_init(|| Regex::new(r"(?i)<br\s*/?>").expect("br"));
    text = br_re.replace_all(&text, "\n").to_string();

    text = tag_strip().replace_all(&text, "").to_string();
    text = decode_entities(&text);

    let multi_nl = MULTI_NL.get_or_init(|| Regex::new(r"\n{3,}").expect("nl"));
    text = multi_nl.replace_all(&text, "\n\n").to_string();
    text.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_cleaner_strips_style_and_keeps_structure() {
        let raw = r#"
        <html>
            <head><style>body { color: red; }</style></head>
            <body>
                <h1>Documentation Title</h1>
                <p>Here is an explanation of <code>asyncio</code> in Python.</p>
                <pre><code>import asyncio
async def main():
    pass</code></pre>
            </body>
        </html>
        "#;
        let md = html_to_markdown(raw);
        assert!(md.contains("# Documentation Title"));
        assert!(md.contains("`asyncio`"));
        assert!(md.contains("```"));
        assert!(!md.contains("<style>"));
    }
}
