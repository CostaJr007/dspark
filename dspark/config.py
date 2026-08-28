"""
Global configuration for the DSpark Dual-Engine platform.
Provides provider-agnostic model routing via LiteLLM, timeouts, and sandbox settings.
"""

from __future__ import annotations

import os
from pathlib import Path
from typing import Optional
from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict


class DualEngineConfig(BaseSettings):
    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        extra="ignore",
    )

    # Creator (High throughput, large context window)
    creator_model: str = Field(
        default="gemini/gemini-2.5-flash",
        description="LiteLLM model identifier for Creator (e.g. gemini/gemini-2.5-flash, claude-3-5-sonnet-20241022, gpt-4o-mini)",
    )
    creator_temperature: float = Field(default=0.2, ge=0.0, le=2.0)

    # Curator (Deep reasoning, mathematical / logical falsification)
    curator_model: str = Field(
        default="deepseek/deepseek-chat",
        description="LiteLLM model identifier for Curator (e.g. deepseek/deepseek-chat, deepseek/deepseek-reasoner, o3-mini)",
    )
    curator_temperature: float = Field(default=0.0, ge=0.0, le=1.0)

    # Refiner (Patching guided by concrete counter-examples)
    refiner_model: str = Field(
        default="deepseek/deepseek-chat",
        description="LiteLLM model identifier for Refiner",
    )
    refiner_temperature: float = Field(default=0.1, ge=0.0, le=1.0)

    # Operational Boundaries & Circuit Breaker
    max_iterations: int = Field(default=3, ge=1, le=10, description="Hard limit for CEGAR refinement passes")
    sandbox_timeout_seconds: int = Field(default=30, ge=5, le=300, description="Pytest sandbox timeout")
    sandbox_temp_dir: Path = Field(
        default_factory=lambda: Path.home() / ".dspark" / "sandbox",
        description="Isolated folder for test code execution",
    )

    # AgentDeltaMemory (KDA-derived agent memory)
    memory_enabled: bool = Field(
        default=True,
        description="Enable AgentDeltaMemory (delta rule, per-channel decay, convergence stop) in the CEGAR loop",
    )
    memory_dim: int = Field(default=64, ge=16, le=512, description="Embedding dimension of the memory state")
    memory_eps: float = Field(default=1e-3, gt=0.0, description="Delta norm below which the memory is converged")

    # Verification scaling (LLM-as-a-Verifier, arXiv:2607.05391)
    curator_repetitions: int = Field(
        default=1,
        ge=1,
        le=8,
        description="Repeated evaluation K: number of independent curator/sandbox audits averaged per CEGAR iteration (variance reduction)",
    )
    voc_stagnation_min_points: int = Field(
        default=3,
        ge=2,
        description="Minimum history points before the VOC (Value-Order Correlation) stagnation stop can fire",
    )
    voc_stagnation_threshold: float = Field(
        default=0.1,
        description="Spearman rank correlation between iteration and score below which the loop is declared stagnant",
    )

    # API Keys (read from environment or .env)
    gemini_api_key: Optional[str] = Field(default=None, alias="GEMINI_API_KEY")
    deepseek_api_key: Optional[str] = Field(default=None, alias="DEEPSEEK_API_KEY")
    anthropic_api_key: Optional[str] = Field(default=None, alias="ANTHROPIC_API_KEY")
    openai_api_key: Optional[str] = Field(default=None, alias="OPENAI_API_KEY")


config = DualEngineConfig()
