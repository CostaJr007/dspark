"""
DSpark - Dual-LLM Speculative Arbitration Engine
High-Throughput Code Generation (Gemini) + Deep Reasoning I/O Arbitration (DeepSeek)
"""

__version__ = "0.1.0"
__author__ = "Adeilson Costa (CostaJr007)"

from .client import DeepSeekClient, GeminiClient, OpenAIClient, LocalLLMClient
from .curator import DeepSeekCurator, CurationVerdict, AuditResult, RefineResult, ArbitrationResult, CounterExample
from .generator import GeminiGenerator
from .pipeline import DSparkPipeline
from .agent import DSparkAgent
from .search import WebSearchEngine, HTMLToMarkdownParser, SearchResult
from .benchmark import DSparkBenchmarkRunner, HumanEvalTask, BenchmarkReport
from .prompts import (
    CURATOR_SYSTEM_PROMPT,
    ARBITRATOR_SYSTEM_PROMPT,
    REFINER_SYSTEM_PROMPT,
    METACOGNITIVE_ENGINEERING_PROMPT,
)

__all__ = [
    "DeepSeekClient",
    "GeminiClient",
    "OpenAIClient",
    "LocalLLMClient",
    "DeepSeekCurator",
    "GeminiGenerator",
    "DSparkPipeline",
    "DSparkAgent",
    "WebSearchEngine",
    "HTMLToMarkdownParser",
    "SearchResult",
    "DSparkBenchmarkRunner",
    "HumanEvalTask",
    "BenchmarkReport",
    "CurationVerdict",
    "AuditResult",
    "RefineResult",
    "ArbitrationResult",
    "CounterExample",
    "CURATOR_SYSTEM_PROMPT",
    "ARBITRATOR_SYSTEM_PROMPT",
    "REFINER_SYSTEM_PROMPT",
    "METACOGNITIVE_ENGINEERING_PROMPT",
]
