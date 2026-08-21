"""
Client wrappers for DeepSeek and Gemini APIs with zero external dependencies.
"""

import json
import os
import urllib.request
import urllib.error
from typing import Any, Dict, List, Optional


class APIError(Exception):
    """Raised when an upstream API returns an error response."""
    def __init__(self, status_code: int, message: str, raw_body: str = ""):
        super().__init__(f"API Error {status_code}: {message}")
        self.status_code = status_code
        self.message = message
        self.raw_body = raw_body


class DeepSeekClient:
    """Zero-dependency HTTP client for DeepSeek Chat and Reasoner APIs."""

    DEFAULT_BASE_URL = "https://api.deepseek.com"
    DEFAULT_MODEL = "deepseek-v4-pro"

    def __init__(
        self,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        default_model: Optional[str] = None,
        timeout: int = 120,
    ):
        self.api_key = api_key or os.environ.get("DEEPSEEK_API_KEY")
        if not self.api_key:
            raise ValueError(
                "DEEPSEEK_API_KEY must be provided or set in environment variables."
            )
        self.base_url = (base_url or os.environ.get("DEEPSEEK_BASE_URL") or self.DEFAULT_BASE_URL).rstrip("/")
        self.default_model = default_model or os.environ.get("DEEPSEEK_MODEL") or self.DEFAULT_MODEL
        self.timeout = timeout

    def chat_completion(
        self,
        messages: List[Dict[str, str]],
        model: Optional[str] = None,
        temperature: float = 0.2,
        max_tokens: int = 4096,
        response_format: Optional[Dict[str, str]] = None,
    ) -> Dict[str, Any]:
        """Send a chat completion request to the DeepSeek API."""
        url = f"{self.base_url}/chat/completions"
        payload = {
            "model": model or self.default_model,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
        }
        if response_format:
            payload["response_format"] = response_format

        data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            url,
            data=data,
            headers={
                "Authorization": f"Bearer {self.api_key}",
                "Content-Type": "application/json",
                "User-Agent": "DSpark-Curator/0.1.0",
            },
            method="POST",
        )

        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                raw_bytes = resp.read()
                return json.loads(raw_bytes.decode("utf-8"))
        except urllib.error.HTTPError as e:
            raw_err = e.read().decode("utf-8", errors="replace")
            try:
                err_json = json.loads(raw_err)
                err_msg = err_json.get("error", {}).get("message", raw_err)
            except Exception:
                err_msg = raw_err
            raise APIError(e.code, err_msg, raw_err) from e
        except urllib.error.URLError as e:
            raise APIError(0, f"Network connection failure: {e.reason}") from e

    def complete(
        self,
        prompt: str,
        system_prompt: Optional[str] = None,
        model: Optional[str] = None,
        temperature: float = 0.2,
        response_format: Optional[Dict[str, str]] = None,
    ) -> str:
        """Convenience method to execute a prompt and return text output."""
        messages = []
        if system_prompt:
            messages.append({"role": "system", "content": system_prompt})
        messages.append({"role": "user", "content": prompt})

        res = self.chat_completion(
            messages,
            model=model,
            temperature=temperature,
            response_format=response_format,
        )
        choices = res.get("choices", [])
        if not choices:
            raise APIError(500, "Empty choices returned by DeepSeek API")
        msg = choices[0].get("message", {})
        content = msg.get("content") or ""
        if not content and msg.get("reasoning_content"):
            content = msg.get("reasoning_content") or ""
        return content


class GeminiClient:
    """Client for Google Gemini API for fast generation phase."""

    DEFAULT_MODEL = "gemini-2.5-flash"

    def __init__(
        self,
        api_key: Optional[str] = None,
        default_model: Optional[str] = None,
        timeout: int = 120,
    ):
        self.api_key = api_key or os.environ.get("GEMINI_API_KEY")
        self.default_model = default_model or os.environ.get("GEMINI_MODEL") or self.DEFAULT_MODEL
        self.timeout = timeout

    def generate_content(
        self,
        prompt: str,
        system_instruction: Optional[str] = None,
        model: Optional[str] = None,
        temperature: float = 0.7,
    ) -> str:
        """Generate content via Gemini REST API."""
        if not self.api_key:
            raise ValueError(
                "GEMINI_API_KEY must be provided or set in environment variables to use GeminiGenerator directly."
            )
        
        target_model = model or self.default_model
        url = f"https://generativelanguage.googleapis.com/v1beta/models/{target_model}:generateContent?key={self.api_key}"
        
        contents = []
        if system_instruction:
            contents.append({
                "role": "user",
                "parts": [{"text": f"System Directive:\n{system_instruction}"}]
            })
            contents.append({
                "role": "model",
                "parts": [{"text": "Understood. I will adhere strictly to these directives."}]
            })

        contents.append({
            "role": "user",
            "parts": [{"text": prompt}]
        })

        payload = {
            "contents": contents,
            "generationConfig": {
                "temperature": temperature,
            }
        }

        data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            url,
            data=data,
            headers={
                "Content-Type": "application/json",
                "User-Agent": "DSpark-Generator/0.1.0",
            },
            method="POST",
        )

        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                raw_bytes = resp.read()
                data_json = json.loads(raw_bytes.decode("utf-8"))
                candidates = data_json.get("candidates", [])
                if not candidates:
                    raise APIError(500, "Empty candidates returned by Gemini API")
                parts = candidates[0].get("content", {}).get("parts", [])
                return "".join(p.get("text", "") for p in parts)
        except urllib.error.HTTPError as e:
            raw_err = e.read().decode("utf-8", errors="replace")
            raise APIError(e.code, raw_err, raw_err) from e


class OpenAIClient:
    """Client for OpenAI Chat Completions API."""

    DEFAULT_BASE_URL = "https://api.openai.com/v1"
    DEFAULT_MODEL = "gpt-4o-mini"

    def __init__(
        self,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        default_model: Optional[str] = None,
        timeout: int = 120,
    ):
        self.api_key = api_key or os.environ.get("OPENAI_API_KEY")
        if not self.api_key:
            raise ValueError("OPENAI_API_KEY must be provided or set in environment variables.")
        self.base_url = (base_url or os.environ.get("OPENAI_BASE_URL") or self.DEFAULT_BASE_URL).rstrip("/")
        self.default_model = default_model or os.environ.get("OPENAI_MODEL") or self.DEFAULT_MODEL
        self.timeout = timeout

    def complete(
        self,
        prompt: str,
        system_prompt: Optional[str] = None,
        model: Optional[str] = None,
        temperature: float = 0.2,
        response_format: Optional[Dict[str, str]] = None,
    ) -> str:
        url = f"{self.base_url}/chat/completions"
        messages = []
        if system_prompt:
            messages.append({"role": "system", "content": system_prompt})
        messages.append({"role": "user", "content": prompt})

        payload = {
            "model": model or self.default_model,
            "messages": messages,
            "temperature": temperature,
        }
        if response_format:
            payload["response_format"] = response_format

        data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            url,
            data=data,
            headers={
                "Authorization": f"Bearer {self.api_key}",
                "Content-Type": "application/json",
                "User-Agent": "DSpark-OpenAI/0.1.0",
            },
            method="POST",
        )

        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                raw_bytes = resp.read()
                data_json = json.loads(raw_bytes.decode("utf-8"))
                choices = data_json.get("choices", [])
                if not choices:
                    raise APIError(500, "Empty choices returned by OpenAI API")
                return choices[0]["message"]["content"]
        except urllib.error.HTTPError as e:
            raw_err = e.read().decode("utf-8", errors="replace")
            raise APIError(e.code, raw_err, raw_err) from e


def create_model_client(model_or_provider: str):
    """
    Factory function resolving model identifier (e.g. 'gpt-4o-mini', 'deepseek-v4-flash', 'deepseek-v4-pro', 'gemini-2.5-flash').
    """
    spec = model_or_provider.lower().strip()
    if spec.startswith("openai:") or "gpt-" in spec:
        model_name = spec.split(":", 1)[1] if ":" in spec else spec
        return OpenAIClient(default_model=model_name)
    elif spec.startswith("gemini:") or "gemini" in spec:
        model_name = spec.split(":", 1)[1] if ":" in spec else spec
        return GeminiClient(default_model=model_name)
    else:
        model_name = spec.split(":", 1)[1] if ":" in spec else spec
        return DeepSeekClient(default_model=model_name)

