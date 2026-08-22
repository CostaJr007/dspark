"""
Unit tests for DSpark State Models and Circuit Breaker logic.
"""

import pytest
from dspark.state import (
    CounterExample,
    DualEngineState,
    IOContract,
    VerdictEnum,
    AuditResult,
    SandboxExecutionResult,
)


def test_io_contract_creation():
    contract = IOContract(
        function_name="divide",
        preconditions=["b != 0", "isinstance(a, (int, float))"],
        postconditions=["result == a / b"],
        invariants=["isinstance(result, float)"],
    )
    assert contract.function_name == "divide"
    assert len(contract.preconditions) == 2
    assert len(contract.postconditions) == 1


def test_counter_example_creation():
    ce = CounterExample(
        function_name="divide",
        input_data={"a": 10, "b": 0},
        expected_behavior="Raises ZeroDivisionError or ValueError",
        actual_behavior="Unhandled ZeroDivisionError",
        failing_assert_code="assert divide(10, 0) == 'error'",
        traceback="ZeroDivisionError: division by zero",
    )
    assert ce.function_name == "divide"
    assert ce.input_data["b"] == 0
    assert ce.id is not None


def test_dual_engine_state_circuit_breaker():
    state = DualEngineState(
        user_spec="Build a safe division function",
        creator_model="gemini/gemini-2.5-flash",
        curator_model="deepseek/deepseek-chat",
        refiner_model="deepseek/deepseek-chat",
        max_iterations=2,
    )

    assert state.verdict == VerdictEnum.PENDING
    assert not state.is_terminal()
    assert state.iteration == 0

    # Iteration 1
    state.increment_iteration()
    assert state.iteration == 1
    assert not state.is_terminal()

    # Iteration 2 (Hits max_iterations)
    state.increment_iteration()
    assert state.iteration == 2
    assert state.verdict == VerdictEnum.MAX_ITER_REACHED
    assert state.is_terminal()


def test_dual_engine_state_approval():
    state = DualEngineState(
        user_spec="Build a safe division function",
        creator_model="gemini/gemini-2.5-flash",
        curator_model="deepseek/deepseek-chat",
        refiner_model="deepseek/deepseek-chat",
        max_iterations=3,
    )
    state.verdict = VerdictEnum.APPROVED
    assert state.is_terminal()
