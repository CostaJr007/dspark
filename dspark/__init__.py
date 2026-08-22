"""
DSpark - Formal Dual-Engine CEGAR & Adversarial Verification Platform.
High-Throughput Code Generation (Creator) + Epistemically-Isolated Adversarial Verification (Curator & Sandbox).
"""

__version__ = "0.2.0"
__author__ = "Adeilson Costa (CostaJr007)"

from .config import DualEngineConfig, config
from .state import (
    IOContract,
    CounterExample,
    DualEngineState,
    AuditResult,
    VerdictEnum,
    SandboxExecutionResult,
)
from .compiler.parser import extract_functions_and_docstrings, infer_contracts_from_ast, parse_code_ast
from .compiler.test_harness import ContractCompiler
from .sandbox.runner import SandboxRunner
from .engines.creator import CreatorEngine
from .engines.curator import CuratorEngine
from .engines.refiner import RefinerEngine
from .pipeline.cegar import CEGARPipeline, run_cegar_pipeline

# Backwards compatibility wrappers
from .client import DeepSeekClient, GeminiClient, OpenAIClient, LocalLLMClient
from .curator import DeepSeekCurator, CurationVerdict, RefineResult, ArbitrationResult
from .generator import GeminiGenerator
from .pipeline import DSparkPipeline
from .agent import DSparkAgent

__all__ = [
    "DualEngineConfig",
    "config",
    "IOContract",
    "CounterExample",
    "DualEngineState",
    "AuditResult",
    "VerdictEnum",
    "SandboxExecutionResult",
    "extract_functions_and_docstrings",
    "infer_contracts_from_ast",
    "parse_code_ast",
    "ContractCompiler",
    "SandboxRunner",
    "CreatorEngine",
    "CuratorEngine",
    "RefinerEngine",
    "CEGARPipeline",
    "run_cegar_pipeline",
    "DeepSeekClient",
    "GeminiClient",
    "OpenAIClient",
    "LocalLLMClient",
    "DeepSeekCurator",
    "GeminiGenerator",
    "DSparkPipeline",
    "DSparkAgent",
]
