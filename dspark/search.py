"""
Kimi-style Web Search and Deep URL Document Reader for DSpark.
Zero-dependency implementation using standard library urllib and HTML parsing.
"""

from dataclasses import dataclass
import html
import json
import re
import urllib.parse
import urllib.request
from typing import List, Optional


@dataclass
class SearchResult:
    title: str
    url: str
    snippet: str


class HTMLToMarkdownParser:
    """Lightweight HTML-to-Markdown converter for clean documentation extraction."""

    @staticmethod
    def clean_html(raw_html: str) -> str:
        # Remove scripts, styles, and SVG
        text = re.sub(r"<(script|style|svg|noscript)[^>]*>.*?</\1>", "", raw_html, flags=re.DOTALL | re.IGNORECASE)
        
        # Convert headers
        for i in range(6, 0, -1):
            text = re.sub(rf"<h{i}[^>]*>(.*?)</h{i}>", rf"\n{'#' * i} \1\n", text, flags=re.DOTALL | re.IGNORECASE)

        # Convert code blocks
        text = re.sub(r"<pre[^>]*><code[^>]*>(.*?)</code></pre>", r"\n```\n\1\n```\n", text, flags=re.DOTALL | re.IGNORECASE)
        text = re.sub(r"<code[^>]*>(.*?)</code>", r"`\1`", text, flags=re.DOTALL | re.IGNORECASE)

        # Convert lists
        text = re.sub(r"<li[^>]*>(.*?)</li>", r"\n* \1", text, flags=re.DOTALL | re.IGNORECASE)

        # Convert paragraphs and breaks
        text = re.sub(r"<p[^>]*>(.*?)</p>", r"\n\1\n", text, flags=re.DOTALL | re.IGNORECASE)
        text = re.sub(r"<br\s*/?>", "\n", text, flags=re.IGNORECASE)

        # Strip remaining tags
        text = re.sub(r"<[^>]+>", "", text)

        # Unescape HTML entities
        text = html.unescape(text)

        # Collapse excess whitespace
        text = re.sub(r"\n{3,}", "\n\n", text)
        return text.strip()


class WebSearchEngine:
    """
    Search engine aggregator that queries the web and parses documentation
    (Inspired by Kimi Code CLI's WebSearch & FetchURL tools).
    """

    USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 DSpark/0.1.0"

    def search(self, query: str, max_results: int = 5, timeout: int = 15) -> List[SearchResult]:
        """
        Perform a web search for documentation, code samples, or error tracebacks.
        Uses DuckDuckGo HTML endpoint with zero external API dependencies.
        """
        encoded_query = urllib.parse.urlencode({"q": query})
        url = f"https://html.duckduckgo.com/html/?{encoded_query}"

        req = urllib.request.Request(
            url,
            headers={
                "User-Agent": self.USER_AGENT,
                "Accept-Language": "en-US,en;q=0.9,pt-BR;q=0.8,pt;q=0.7",
            },
        )

        results: List[SearchResult] = []

        try:
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                content = resp.read().decode("utf-8", errors="replace")

            # Extract result blocks
            blocks = re.findall(r'<div class="result__body">(.*?)</div>\s*</div>', content, re.DOTALL)
            for block in blocks[:max_results]:
                # Extract Title and URL
                title_match = re.search(r'<a class="result__url"[^>]*href="([^"]+)"[^>]*>(.*?)</a>', block, re.DOTALL)
                snippet_match = re.search(r'<a class="result__snippet"[^>]*>(.*?)</a>', block, re.DOTALL)

                if not title_match:
                    title_match = re.search(r'<a class="result__a"[^>]*href="([^"]+)"[^>]*>(.*?)</a>', block, re.DOTALL)

                if title_match:
                    raw_href = title_match.group(1)
                    # Extract target URL from uddg redirect
                    parsed = urllib.parse.urlparse(raw_href)
                    qs = urllib.parse.parse_qs(parsed.query)
                    target_url = qs.get("uddg", [raw_href])[0]

                    title = re.sub(r"<[^>]+>", "", title_match.group(2)).strip()
                    snippet = re.sub(r"<[^>]+>", "", snippet_match.group(1)).strip() if snippet_match else ""

                    results.append(
                        SearchResult(
                            title=html.unescape(title),
                            url=target_url,
                            snippet=html.unescape(snippet),
                        )
                    )

        except Exception as e:
            # Fallback mock/safe result if network is restricted
            results.append(
                SearchResult(
                    title=f"Web search query for: {query}",
                    url=f"https://www.google.com/search?{encoded_query}",
                    snippet=f"Search performed. (Notice: Direct fetch failed with {e})",
                )
            )

        return results

    def fetch_url(self, url: str, timeout: int = 15, max_chars: int = 8000) -> str:
        """
        Fetch and parse a webpage, converting it to clean Markdown for model consumption.
        """
        req = urllib.request.Request(
            url,
            headers={"User-Agent": self.USER_AGENT},
        )

        try:
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                raw_html = resp.read().decode("utf-8", errors="replace")

            markdown = HTMLToMarkdownParser.clean_html(raw_html)
            if len(markdown) > max_chars:
                return markdown[:max_chars] + "\n\n... [Content truncated for context window] ..."
            return markdown
        except Exception as e:
            return f"Failed to fetch content from {url}: {e}"

    def research_topic(self, topic: str, max_sources: int = 3) -> str:
        """
        Perform a multi-step research on a topic: searches and fetches top source contents.
        """
        search_results = self.search(topic, max_results=max_sources)
        if not search_results:
            return f"No results found for topic: {topic}"

        report = [f"## Web Research Results for: '{topic}'\n"]
        for idx, res in enumerate(search_results, 1):
            report.append(f"### Source {idx}: [{res.title}]({res.url})")
            report.append(f"**Snippet**: {res.snippet}\n")
            # Fetch snippet of documentation
            page_text = self.fetch_url(res.url, max_chars=1500)
            report.append(f"**Page Excerpt**:\n```\n{page_text[:800]}\n```\n")

        return "\n".join(report)
