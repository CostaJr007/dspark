"""
Fast Generator Engine for DSpark (Gemini integration).
"""

from typing import Optional
from .client import GeminiClient


DRAFT_SYSTEM_INSTRUCTION = """You are a high-speed software development assistant.
Generate clean, idiomatic, and complete code implementations according to the requested prompt.
Do not omit details or leave placeholders (like '// TODO' or 'pass') unless explicitly requested.
Wrap your output in standard markdown code blocks.
"""


class GeminiGenerator:
    """
    High-throughput draft code generator powered by Google Gemini.
    """

    def __init__(self, client: Optional[GeminiClient] = None, model: Optional[str] = None):
        self.client = client or GeminiClient()
        self.model = model or self.client.default_model

    def generate_draft(
        self,
        prompt: str,
        language: Optional[str] = None,
        temperature: float = 0.7,
    ) -> str:
        """
        Generate a fast, high-throughput code draft from a natural language specification.
        """
        lang_directive = f" (Target language: {language})" if language else ""
        full_prompt = f"Implement the following feature or function{lang_directive}:\n\n{prompt}"

        return self.client.generate_content(
            prompt=full_prompt,
            system_instruction=DRAFT_SYSTEM_INSTRUCTION,
            model=self.model,
            temperature=temperature,
        )
