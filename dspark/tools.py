"""
DSpark agentic tool registry (file, shell, search, curator).
"""

import json
import os
import subprocess
from dataclasses import dataclass
from typing import Any, Callable, Dict, List, Optional

from .curator import DeepSeekCurator
from .search import WebSearchEngine


@dataclass
class ToolResult:
    tool_name: str
    success: bool
    output: str
    error: Optional[str] = None


class ToolRegistry:
    """
    Registry of system and coding tools executable by the DSpark agent.
    """

    def __init__(self, working_dir: str, curator: Optional[DeepSeekCurator] = None):
        self.working_dir = os.path.abspath(working_dir)
        self.curator = curator or DeepSeekCurator()
        self.search_engine = WebSearchEngine()

    def read_file(self, path: str, start_line: Optional[int] = None, end_line: Optional[int] = None) -> ToolResult:
        full_path = os.path.join(self.working_dir, path) if not os.path.isabs(path) else path
        if not os.path.exists(full_path):
            return ToolResult("read_file", False, "", f"File '{path}' does not exist.")
        try:
            with open(full_path, "r", encoding="utf-8", errors="replace") as f:
                lines = f.readlines()
            if start_line is not None or end_line is not None:
                s = max(0, (start_line or 1) - 1)
                e = min(len(lines), end_line or len(lines))
                content = "".join(lines[s:e])
            else:
                content = "".join(lines)
            return ToolResult("read_file", True, content)
        except Exception as e:
            return ToolResult("read_file", False, "", str(e))

    def write_file(self, path: str, content: str) -> ToolResult:
        full_path = os.path.join(self.working_dir, path) if not os.path.isabs(path) else path
        try:
            os.makedirs(os.path.dirname(os.path.abspath(full_path)), exist_ok=True)
            with open(full_path, "w", encoding="utf-8") as f:
                f.write(content)
            return ToolResult("write_file", True, f"Successfully wrote {len(content)} bytes to {path}")
        except Exception as e:
            return ToolResult("write_file", False, "", str(e))

    def edit_file(self, path: str, target_chunk: str, replacement_chunk: str) -> ToolResult:
        full_path = os.path.join(self.working_dir, path) if not os.path.isabs(path) else path
        if not os.path.exists(full_path):
            return ToolResult("edit_file", False, "", f"File '{path}' does not exist.")
        try:
            with open(full_path, "r", encoding="utf-8") as f:
                content = f.read()
            if target_chunk not in content:
                return ToolResult("edit_file", False, "", f"Target chunk not found in {path}")
            new_content = content.replace(target_chunk, replacement_chunk, 1)
            with open(full_path, "w", encoding="utf-8") as f:
                f.write(new_content)
            return ToolResult("edit_file", True, f"Successfully edited {path}")
        except Exception as e:
            return ToolResult("edit_file", False, "", str(e))

    def list_files(self, relative_path: str = ".") -> ToolResult:
        target_dir = os.path.join(self.working_dir, relative_path)
        if not os.path.exists(target_dir):
            return ToolResult("list_files", False, "", f"Directory '{relative_path}' does not exist.")
        entries = []
        for root, _, files in os.walk(target_dir):
            if any(p in root for p in [".git", "__pycache__", "node_modules", ".venv"]):
                continue
            for file in files:
                rel = os.path.relpath(os.path.join(root, file), self.working_dir)
                entries.append(rel)
        return ToolResult("list_files", True, "\n".join(sorted(entries)[:120]))

    def run_terminal(self, command: str, timeout: int = 90) -> ToolResult:
        try:
            res = subprocess.run(
                command,
                shell=True,
                cwd=self.working_dir,
                capture_output=True,
                text=True,
                timeout=timeout,
            )
            out = f"Exit code: {res.returncode}\n"
            if res.stdout:
                out += f"STDOUT:\n{res.stdout}\n"
            if res.stderr:
                out += f"STDERR:\n{res.stderr}\n"
            return ToolResult("run_terminal", res.returncode == 0, out.strip())
        except subprocess.TimeoutExpired:
            return ToolResult("run_terminal", False, "", f"Command timed out after {timeout} seconds.")
        except Exception as e:
            return ToolResult("run_terminal", False, "", str(e))

    def search_web(self, query: str) -> ToolResult:
        try:
            results = self.search_engine.search(query, max_results=5)
            formatted = [f"Web search results for: '{query}':"]
            for idx, r in enumerate(results, 1):
                formatted.append(f"{idx}. {r.title} ({r.url})\n   {r.snippet}")
            return ToolResult("search_web", True, "\n\n".join(formatted))
        except Exception as e:
            return ToolResult("search_web", False, "", str(e))

    def verify_with_curator(self, code: str, specification: str) -> ToolResult:
        try:
            audit_res = self.curator.audit(code=code, specification=specification)
            summary = (
                f"Curator Verdict: {audit_res.verdict.value} (Score: {audit_res.score}/100)\n"
                f"Summary: {audit_res.summary}\n"
                f"Counter-Examples Detected: {len(audit_res.counter_examples)}"
            )
            return ToolResult("verify_with_curator", audit_res.is_approved, summary)
        except Exception as e:
            return ToolResult("verify_with_curator", False, "", str(e))
