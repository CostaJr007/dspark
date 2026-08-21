"""
Autonomous Agent Runtime for DSpark (inspired by Grok Build + Dual-Engine Verification).
"""

import os
import subprocess
import sys
from typing import Any, Dict, List, Optional

from .client import DeepSeekClient
from .curator import DeepSeekCurator
from .prompts import METACOGNITIVE_ENGINEERING_PROMPT
from .search import WebSearchEngine


class DSparkAgent:
    """
    Autonomous terminal coding agent (inspired by Grok Build & Kimi Code),
    with built-in metacognitive reasoning, web research, terminal tools,
    and formal verification loops.
    """

    def __init__(
        self,
        working_dir: Optional[str] = None,
        model: Optional[str] = None,
        curator: Optional[DeepSeekCurator] = None,
        search_engine: Optional[WebSearchEngine] = None,
    ):
        self.working_dir = working_dir or os.getcwd()
        self.client = DeepSeekClient(default_model=model)
        self.curator = curator or DeepSeekCurator()
        self.search_engine = search_engine or WebSearchEngine()
        self.history: List[Dict[str, str]] = []

    def read_file(self, path: str) -> str:
        """Read text content from a workspace file."""
        full_path = os.path.join(self.working_dir, path) if not os.path.isabs(path) else path
        if not os.path.exists(full_path):
            raise FileNotFoundError(f"File '{path}' does not exist.")
        with open(full_path, "r", encoding="utf-8") as f:
            return f.read()

    def write_file(self, path: str, content: str) -> str:
        """Write content safely to a workspace file."""
        full_path = os.path.join(self.working_dir, path) if not os.path.isabs(path) else path
        os.makedirs(os.path.dirname(os.path.abspath(full_path)), exist_ok=True)
        with open(full_path, "w", encoding="utf-8") as f:
            f.write(content)
        return f"Successfully wrote {len(content)} bytes to {path}"

    def list_files(self, relative_path: str = ".") -> List[str]:
        """List files in the specified directory."""
        target_dir = os.path.join(self.working_dir, relative_path)
        if not os.path.exists(target_dir):
            return []
        entries = []
        for root, _, files in os.walk(target_dir):
            if any(p in root for p in [".git", "__pycache__", "node_modules", ".venv"]):
                continue
            for file in files:
                rel = os.path.relpath(os.path.join(root, file), self.working_dir)
                entries.append(rel)
        return sorted(entries)[:100]

    def search_web(self, query: str, max_results: int = 5) -> str:
        """Perform a web search for documentation or solutions (Kimi Code style)."""
        results = self.search_engine.search(query, max_results=max_results)
        if not results:
            return f"No web search results found for: {query}"
        output = [f"Web search results for '{query}':\n"]
        for idx, res in enumerate(results, 1):
            output.append(f"{idx}. {res.title}\n   URL: {res.url}\n   {res.snippet}\n")
        return "\n".join(output)

    def fetch_url(self, url: str) -> str:
        """Fetch and convert a documentation page to clean Markdown."""
        return self.search_engine.fetch_url(url)

    def run_terminal(self, command: str, timeout: int = 60) -> str:
        """Execute a shell command locally (e.g. pytest, npm test, cargo check)."""
        try:
            res = subprocess.run(
                command,
                shell=True,
                cwd=self.working_dir,
                capture_output=True,
                text=True,
                timeout=timeout,
            )
            output = f"Exit code: {res.returncode}\n"
            if res.stdout:
                output += f"STDOUT:\n{res.stdout}\n"
            if res.stderr:
                output += f"STDERR:\n{res.stderr}\n"
            return output.strip()
        except subprocess.TimeoutExpired:
            return f"Command timed out after {timeout} seconds."
        except Exception as e:
            return f"Command execution error: {e}"

    def execute_task(self, user_instruction: str) -> str:
        """
        Execute an engineering task enforcing the Metacognitive Reasoning Protocol.
        """
        messages = [
            {"role": "system", "content": METACOGNITIVE_ENGINEERING_PROMPT},
            {"role": "user", "content": f"Working Directory: {self.working_dir}\n\nTask: {user_instruction}"},
        ]

        response = self.client.chat_completion(messages)
        content = response["choices"][0]["message"]["content"]
        return content
