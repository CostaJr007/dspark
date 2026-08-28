"""
Core state models, contracts, and counterexample definitions for DSpark CEGAR Dual-Engine.
"""

from __future__ import annotations

from enum import Enum
import uuid
from typing import Any, Dict, List, Literal, Optional
from pydantic import BaseModel, Field


class VerdictEnum(str, Enum):
    PENDING = "PENDING"
    APPROVED = "APPROVED"
    REJECTED = "REJECTED"
    MAX_ITER_REACHED = "MAX_ITER_REACHED"
    ERROR = "ERROR"


class IOContract(BaseModel):
    """
    Formal Design-by-Contract (DbC) specification for a target function/method.
    """
    function_name: str = Field(description="Exact function/method name in the AST")
    preconditions: List[str] = Field(
        default_factory=list,
        description="Boolean expressions that MUST hold before execution (e.g. 'x > 0', 'isinstance(arr, list)')",
    )
    postconditions: List[str] = Field(
        default_factory=list,
        description="Boolean expressions guaranteed to hold after execution (e.g. 'result >= 0', 'len(result) == len(arr)')",
    )
    invariants: List[str] = Field(
        default_factory=list,
        description="Properties and constraints that remain true throughout function lifecycle",
    )


class CounterExample(BaseModel):
    """
    Concrete, deterministic counterexample synthesized by Curator or verified by Sandbox.
    """
    id: str = Field(default_factory=lambda: str(uuid.uuid4())[:8])
    function_name: str = Field(default="target_function", description="Target function under test")
    input_data: Dict[str, Any] = Field(
        default_factory=dict,
        description="Exact concrete parameter values that trigger the failure",
    )
    expected_behavior: str = Field(
        default="",
        description="Expected postcondition, return value, or handled exception",
    )
    actual_behavior: str = Field(
        default="",
        description="Actual observed behavior (e.g. IndexError, wrong return, AssertionError)",
    )
    failing_assert_code: str = Field(
        default="",
        description="The exact executable Python assertion / pytest line that failed",
    )
    traceback: Optional[str] = Field(
        default=None,
        description="Pytest/sandbox stderr and traceback capture",
    )


class SandboxExecutionResult(BaseModel):
    """
    Direct operating-system level execution result from the Sandbox runner.
    """
    exit_code: int = Field(description="Subprocess return code (0 = success, !=0 = failure)")
    stdout: str = Field(default="")
    stderr: str = Field(default="")
    passed_tests: int = Field(default=0)
    failed_tests: int = Field(default=0)
    duration_seconds: float = Field(default=0.0)
    timed_out: bool = Field(default=False)
    counter_examples: List[CounterExample] = Field(default_factory=list)


class AuditResult(BaseModel):
    """
    Formal verdict and artifacts produced by the Curator & Sandbox verification pass.
    """
    verdict: VerdictEnum = Field(default=VerdictEnum.PENDING)
    score: int = Field(default=0, ge=0, le=100)
    summary: str = Field(default="")
    contracts: List[IOContract] = Field(default_factory=list)
    counter_examples: List[CounterExample] = Field(default_factory=list)
    generated_tests: str = Field(default="", description="Pytest test suite code")
    sandbox_result: Optional[SandboxExecutionResult] = None
    critical_issues: List[str] = Field(default_factory=list)
    suggested_improvements: List[str] = Field(default_factory=list)
    refined_code: Optional[str] = None
    raw_response: str = Field(default="")
    criteria_scores: Dict[str, int] = Field(
        default_factory=dict,
        description="Per-criterion scores from the verifier's criteria decomposition "
        "(Specification / Output / Errors, per LLM-as-a-Verifier Section 4.3)",
    )

    @property
    def is_approved(self) -> bool:
        return self.verdict == VerdictEnum.APPROVED


class DualEngineState(BaseModel):
    """
    Global state machine object driving the CEGAR Dual-Engine lifecycle.
    """
    task_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    user_spec: str = Field(
        description="Original user specification / task description (visible ONLY to Creator)",
    )
    language: str = Field(default="python", description="Target programming language")

    # Allocated model engines
    creator_model: str
    curator_model: str
    refiner_model: str

    # Pipeline Artifacts
    current_draft: Optional[str] = Field(default=None, description="Current code implementation draft")
    contracts: List[IOContract] = Field(default_factory=list, description="Formal I/O contracts")
    test_harness_code: Optional[str] = Field(default=None, description="Executable test harness code")
    curator_test_code: Optional[str] = Field(default=None, description="Adversarial tests from Curator")

    # State & Verdict
    verdict: VerdictEnum = Field(default=VerdictEnum.PENDING)
    counter_examples: List[CounterExample] = Field(default_factory=list)
    iteration: int = Field(default=0, ge=0)
    max_iterations: int = Field(default=3)
    history: List[Dict[str, Any]] = Field(default_factory=list)
    error_message: Optional[str] = None

    # AgentDeltaMemory observability
    memory_stats: Dict[str, Any] = Field(
        default_factory=dict,
        description="AgentDeltaMemory statistics captured at pipeline end (KDA-derived)",
    )
    memory_stable: bool = Field(
        default=False,
        description="True when the delta rule converged (delta < eps): refinement loop stopped learning",
    )

    # Verifier progress signal (LLM-as-a-Verifier, Section 6)
    voc: Optional[float] = Field(
        default=None,
        description="Latest Value-Order Correlation: Spearman rank correlation between "
        "CEGAR iteration index and curator score (task-progress proxy)",
    )
    voc_stagnated: bool = Field(
        default=False,
        description="True when the loop stopped early because the score stopped improving (VOC below threshold)",
    )

    def increment_iteration(self) -> None:
        """Advance iteration and trigger circuit breaker if max reached."""
        self.iteration += 1
        if self.iteration >= self.max_iterations and self.verdict != VerdictEnum.APPROVED:
            self.verdict = VerdictEnum.MAX_ITER_REACHED

    def is_terminal(self) -> bool:
        """Check if terminal state is reached."""
        return self.verdict in (
            VerdictEnum.APPROVED,
            VerdictEnum.MAX_ITER_REACHED,
            VerdictEnum.ERROR,
        )
