"""
Autonomous Agent Runtime for DSpark (inspired by Grok Build + Dual-Engine Verification).
"""

import os
import subprocess
import sys
from typing import Dict, List, Optional

from .client import DeepSeekClient
from .curator import DeepSeekCurator
from .prompts import METACOGNITIVE_ENGINEERING_PROMPT


class DSparkAgent:
    """
    Autonomous terminal coding agent with built-in metacognitive reasoning,
    local file tools, terminal execution, and verification loops.
    """

    def __init__(
        self,
        working_dir: Optional[str] = None,
        model: Optional[str] = None,
        curator: Optional[DeepSeekCurator] = None,
    ):
        self.working_dir = working_dir or os.getcwd()
        self.client = DeepSeekClient(default_model=model)
        self.curator = curator or DeepSeekCurator()
        self.history: List[Dict[str, str]] = []

    def read_file(self, path: str) -> str:
        full_path = os.path.join(self.working_dir, path) if not os.path.isabs(path) else path
        if not os.path.exists(full_path):
            raise FileNotFoundError(f"File '{path}' does not exist.")
        with open(full_path, "r", encoding="utf-8") as f:
            return f.read()

    def write_file(self, path: str, content: str) -> str:
        full_path = os.path.join(self.working_dir, path) if not os.path.isabs(path) else path
        os.makedirs(os.path.dirname(os.path.abspath(full_path)), exist_ok=True)
        with open(full_path, "w", encoding="utf-8") as f:
            f.write(content)
        return f"Successfully wrote {len(content)} bytes to {path}"

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
