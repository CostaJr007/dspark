"""
DSpark / Grok Build Autonomous Agentic Execution Engine.
Fuses Grok Build's full Agentic Tool Execution Loop with DSpark's Speculative Dual-Engine Verification.
"""

import json
import re
from typing import Any, Callable, Dict, List, Optional

from .client import create_model_client
from .curator import DeepSeekCurator
from .prompts import METACOGNITIVE_ENGINEERING_PROMPT
from .tools import ToolRegistry, ToolResult


GROK_BUILD_SYSTEM_PROMPT = """You are DSpark (Grok-Build Enhanced Agent), an autonomous software engineering agent with deep reasoning and tool execution capabilities.

You have access to a local development environment and the following tools:
1. `read_file(path, start_line, end_line)`: Read source code files.
2. `write_file(path, content)`: Create or overwrite files.
3. `edit_file(path, target_chunk, replacement_chunk)`: Surgically replace code chunks.
4. `list_files(relative_path)`: Explore workspace directory tree.
5. `run_terminal(command)`: Run local shell commands (e.g. pytest, git, python).
6. `search_web(query)`: Deep web search for APIs, docs, and error solutions.
7. `verify_with_curator(code, specification)`: Submit code to DeepSeek Reasoning Curator for formal I/O audit.

When addressing user requests:
1. Always follow the 3-step Metacognitive Protocol:
   - Step 1: Análise e Raciocínio (Where, How, Why, I/O Contracts, Specific & Non-specific impact).
   - Step 2: Testes Mentais e Estáticos Realizados (Edge cases & validation).
   - Step 3: Mudanças Propostas (Commented Diff & Implementation).
2. To invoke a tool, output a JSON block formatted exactly as:
```json
{
  "tool": "tool_name",
  "args": { ... }
}
```
If no tool is required, directly provide your clear, expert answer in GitHub-style Markdown.
"""


class GrokAgent:
    """
    Autonomous multi-turn agent that executes tools, plans tasks,
    and runs the DSpark Dual-Engine verification loop.
    """

    def __init__(
        self,
        working_dir: str,
        generator_model: str = "gpt-4o-mini",
        curator_model: str = "deepseek-v4-flash",
    ):
        self.working_dir = working_dir
        self.generator_model = generator_model
        self.curator_model = curator_model

        self.client = create_model_client(generator_model)
        self.curator = DeepSeekCurator(model=curator_model)
        self.tools = ToolRegistry(working_dir=working_dir, curator=self.curator)
        self.conversation_history: List[Dict[str, str]] = []

    def execute_step(
        self,
        user_prompt: str,
        on_tool_call: Optional[Callable[[str, Dict[str, Any]], None]] = None,
        on_tool_result: Optional[Callable[[ToolResult], None]] = None,
        max_iterations: int = 6,
    ) -> str:
        """
        Runs the multi-turn agentic loop until completion or max_iterations reached.
        """
        messages = [
            {"role": "system", "content": GROK_BUILD_SYSTEM_PROMPT},
            {"role": "user", "content": f"Workspace: {self.working_dir}\n\nRequest: {user_prompt}"},
        ]

        current_iteration = 0
        final_answer = ""

        while current_iteration < max_iterations:
            current_iteration += 1

            if hasattr(self.client, "complete"):
                prompt_text = "\n".join(f"{m['role']}: {m['content']}" for m in messages[1:])
                system_text = messages[0]["content"]
                response_text = self.client.complete(prompt=prompt_text, system_prompt=system_text)
            else:
                resp = self.client.chat_completion(messages)
                choices = resp.get("choices", [])
                response_text = choices[0]["message"]["content"] if choices else ""

            # Check if model requested a tool call via JSON block
            tool_match = re.search(r"```json\s*(\{.*?\})\s*```", response_text, re.DOTALL)
            if not tool_match:
                final_answer = response_text
                break

            try:
                tool_data = json.loads(tool_match.group(1))
                tool_name = tool_data.get("tool")
                args = tool_data.get("args", {})

                if on_tool_call:
                    on_tool_call(tool_name, args)

                # Execute tool
                if tool_name == "read_file":
                    res = self.tools.read_file(**args)
                elif tool_name == "write_file":
                    res = self.tools.write_file(**args)
                elif tool_name == "edit_file":
                    res = self.tools.edit_file(**args)
                elif tool_name == "list_files":
                    res = self.tools.list_files(**args)
                elif tool_name == "run_terminal":
                    res = self.tools.run_terminal(**args)
                elif tool_name == "search_web":
                    res = self.tools.search_web(**args)
                elif tool_name == "verify_with_curator":
                    res = self.tools.verify_with_curator(**args)
                else:
                    res = ToolResult(tool_name, False, "", f"Unknown tool '{tool_name}'")

                if on_tool_result:
                    on_tool_result(res)

                # Append tool interaction to context
                messages.append({"role": "assistant", "content": response_text})
                messages.append({
                    "role": "user",
                    "content": f"Tool '{tool_name}' Result (Success: {res.success}):\n{res.output or res.error}",
                })

            except Exception as e:
                messages.append({"role": "assistant", "content": response_text})
                messages.append({"role": "user", "content": f"Tool execution failed: {e}"})

        return final_answer or response_text
