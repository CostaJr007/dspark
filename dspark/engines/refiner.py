"""
Refiner Engine: CEGAR-driven surgical patching based on concrete counterexamples.
"""

from __future__ import annotations

import json
import logging
import re
from typing import List, Optional

import litellm

from ..config import config
from ..state import CounterExample, IOContract

logger = logging.getLogger("dspark.refiner")

REFINER_SYSTEM_PROMPT = """
You are a Surgical Code Refiner operating under formal Counterexample-Guided Abstraction Refinement (CEGAR).

Mission:
Patch the source code to resolve the exact failing assertions, boundary bugs, and counterexamples provided.
Do NOT rewrite unrelated logic. Preserve all pre-existing I/O contracts and signatures.

Input provided:
- Original Source Code
- Formal I/O Contracts
- Concrete Counterexamples (failing inputs, assert statements, and sandbox traceback)

Output format:
Return EXCLUSIVELY the complete corrected Python code enclosed in:
```python
<REFINED_CODE>
```
"""


class RefinerEngine:
    """
    Refiner Engine responsible for patching code based on ground-truth counterexamples.
    """

    def __init__(self, model: Optional[str] = None, temperature: Optional[float] = None):
        self.model = model or config.refiner_model
        self.temperature = temperature if temperature is not None else config.refiner_temperature

    async def refine_code(
        self,
        source_code: str,
        counter_examples: List[CounterExample],
        contracts: Optional[List[IOContract]] = None,
    ) -> str:
        """
        Synthesizes a refined implementation resolving all identified counterexamples.
        """
        # Format counterexamples for prompt
        ce_descriptions = []
        for i, ce in enumerate(counter_examples, start=1):
            ce_descriptions.append(
                f"### Counterexample {i} (Target: {ce.function_name}):\n"
                f"- Failing Input Data: {json.dumps(ce.input_data)}\n"
                f"- Expected Behavior: {ce.expected_behavior}\n"
                f"- Actual Failure: {ce.actual_behavior}\n"
                f"- Failing Code/Assert: `{ce.failing_assert_code}`\n"
                f"- Sandbox Traceback:\n```\n{ce.traceback or 'N/A'}\n```"
            )

        ce_text = "\n\n".join(ce_descriptions)
        contracts_text = json.dumps([c.model_dump() for c in (contracts or [])], indent=2)

        messages = [
            {"role": "system", "content": REFINER_SYSTEM_PROMPT},
            {
                "role": "user",
                "content": (
                    f"### ORIGINAL SOURCE CODE:\n```python\n{source_code}\n```\n\n"
                    f"### I/O CONTRACTS:\n```json\n{contracts_text}\n```\n\n"
                    f"### DETERMINISTIC COUNTEREXAMPLES FROM SANDBOX:\n{ce_text}\n\n"
                    "Generate the complete, patched source code that fixes these counterexamples."
                ),
            },
        ]

        try:
            response = await litellm.acompletion(
                model=self.model,
                messages=messages,
                temperature=self.temperature,
            )
            raw_text = response.choices[0].message.content or ""
            return self._extract_code(raw_text, fallback=source_code)
        except Exception as e:
            logger.warning(f"LiteLLM call failed for Refiner ({self.model}): {e}.")
            return source_code

    def _extract_code(self, text: str, fallback: str) -> str:
        """Extracts code from markdown block."""
        match = re.search(r"```(?:python)?\s*(.*?)\s*```", text, re.DOTALL)
        if match:
            return match.group(1).strip()
        return text.strip() or fallback
