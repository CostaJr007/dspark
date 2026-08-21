"""
DSpark - Dual-LLM Speculative Arbitration Engine
High-Throughput Code Generation (Gemini) + Deep Reasoning I/O Arbitration (DeepSeek)
"""

__version__ = "0.1.0"
__author__ = "Adeilson Costa (CostaJr007)"

from .client import DeepSeekClient, GeminiClient
from .curator import DeepSeekCurator, CurationVerdict, AuditResult, RefineResult, ArbitrationResult
from .generator import GeminiGenerator
from .pipeline import DSparkPipeline
from .agent import DSparkAgent
from .search import WebSearchEngine, HTMLToMarkdownParser, SearchResult
from .prompts import (
    CURATOR_SYSTEM_PROMPT,
    ARBITRATOR_SYSTEM_PROMPT,
    REFINER_SYSTEM_PROMPT,
    METACOGNITIVE_ENGINEERING_PROMPT,
)

__all__ = [
    "DeepSeekClient",
    "GeminiClient",
    "DeepSeekCurator",
    "GeminiGenerator",
    "DSparkPipeline",
    "DSparkAgent",
    "WebSearchEngine",
    "HTMLToMarkdownParser",
    "SearchResult",
    "CurationVerdict",
    "AuditResult",
    "RefineResult",
    "ArbitrationResult",
    "CURATOR_SYSTEM_PROMPT",
    "ARBITRATOR_SYSTEM_PROMPT",
    "REFINER_SYSTEM_PROMPT",
    "METACOGNITIVE_ENGINEERING_PROMPT",
]
