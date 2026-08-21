"""
DSpark - Dual-LLM Speculative Arbitration Engine
High-Throughput Code Generation (Gemini) + Deep Reasoning I/O Arbitration (DeepSeek)
"""

__version__ = "0.1.0"
__author__ = "Adeilton Costa Jr (CostaJr007)"

from .client import DeepSeekClient, GeminiClient
from .curator import DeepSeekCurator, CurationVerdict, AuditResult, RefineResult, ArbitrationResult
from .generator import GeminiGenerator
from .pipeline import DSparkPipeline
from .prompts import CURATOR_SYSTEM_PROMPT, ARBITRATOR_SYSTEM_PROMPT, REFINER_SYSTEM_PROMPT

__all__ = [
    "DeepSeekClient",
    "GeminiClient",
    "DeepSeekCurator",
    "GeminiGenerator",
    "DSparkPipeline",
    "CurationVerdict",
    "AuditResult",
    "RefineResult",
    "ArbitrationResult",
    "CURATOR_SYSTEM_PROMPT",
    "ARBITRATOR_SYSTEM_PROMPT",
    "REFINER_SYSTEM_PROMPT",
]
