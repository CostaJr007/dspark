"""
Creator Engine: High-throughput generation and formal I/O contract extraction.
"""

from __future__ import annotations

import json
import logging
import re
from typing import Any, Dict, List, Optional, Tuple

import litellm

from ..config import config
from ..compiler.parser import infer_contracts_from_ast
from ..state import IOContract

logger = logging.getLogger("dspark.creator")

CREATOR_SYSTEM_PROMPT = """
You are a Senior Software Engineer specializing in Design by Contract (DbC) and formal code generation.

Your mission:
1. Write clean, production-ready, strictly-typed implementation code for the given user specification.
2. Formulate explicit I/O Contracts for every function/class:
   - Preconditions: What MUST be true before execution.
   - Postconditions: What MUST be guaranteed upon return.
   - Invariants: What state properties remain preserved.

Output format:
Return your response containing:
```python
<YOUR_IMPLEMENTATION_CODE>
```

Followed by a JSON block defining the contracts:
```json
[
  {
    "function_name": "exact_name",
    "preconditions": ["valid_boolean_expression_or_condition"],
    "postconditions": ["valid_boolean_expression_or_condition"],
    "invariants": ["state_invariants"]
  }
]
```
"""


class CreatorEngine:
    """
    Creator Engine responsible for drafting code and extracting formal I/O contracts.
    """

    def __init__(self, model: Optional[str] = None, temperature: Optional[float] = None):
        self.model = model or config.creator_model
        self.temperature = temperature if temperature is not None else config.creator_temperature

    async def generate_draft_and_contracts(
        self,
        user_spec: str,
        language: str = "python",
    ) -> Tuple[str, List[IOContract]]:
        """
        Generates implementation code and I/O contracts from user specification.
        """
        messages = [
            {"role": "system", "content": CREATOR_SYSTEM_PROMPT},
            {
                "role": "user",
                "content": f"Language: {language}\nSpecification:\n{user_spec}",
            },
        ]

        try:
            response = await litellm.acompletion(
                model=self.model,
                messages=messages,
                temperature=self.temperature,
            )
            raw_text = response.choices[0].message.content or ""
        except Exception as e:
            logger.warning(f"LiteLLM call failed for {self.model}: {e}. Falling back to default parser/template.")
            raw_text = f"# Draft generated for spec: {user_spec}\n\ndef solve():\n    pass\n"

        code = self._extract_code(raw_text)
        contracts = self._extract_contracts(raw_text, code)

        return code, contracts

    def _extract_code(self, text: str) -> str:
        """Extracts code from markdown blocks or returns cleaned text."""
        code_match = re.search(r"```(?:python)?\s*(.*?)\s*```", text, re.DOTALL)
        if code_match:
            # Check if this block is JSON, if so search for python block
            content = code_match.group(1).strip()
            if not content.startswith("[") and not content.startswith("{"):
                return content

        # Look specifically for ```python ... ```
        py_match = re.search(r"```python\s*(.*?)\s*```", text, re.DOTALL)
        if py_match:
            return py_match.group(1).strip()

        # Fallback to entire text if no markdown fences
        return text.strip()

    def _extract_contracts(self, text: str, code: str) -> List[IOContract]:
        """Extracts JSON contract array or falls back to AST inference."""
        json_matches = re.findall(r"```(?:json)?\s*(\[.*?\])\s*```", text, re.DOTALL)
        for match in json_matches:
            try:
                data = json.loads(match)
                if isinstance(data, list):
                    return [IOContract(**item) for item in data if isinstance(item, dict)]
            except Exception:
                continue

        # Direct JSON find
        start_bracket = text.find("[")
        end_bracket = text.rfind("]")
        if start_bracket != -1 and end_bracket > start_bracket:
            try:
                data = json.loads(text[start_bracket : end_bracket + 1])
                if isinstance(data, list):
                    return [IOContract(**item) for item in data if isinstance(item, dict)]
            except Exception:
                pass

        # Fallback: Infer contracts directly from Python AST
        return infer_contracts_from_ast(code)
