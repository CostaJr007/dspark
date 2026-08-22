"""
Unit tests for the isolated Subprocess Sandbox Runner.
"""

from dspark.sandbox.runner import SandboxRunner


def test_sandbox_passing_execution():
    runner = SandboxRunner()
    
    source = """
def add(a: int, b: int) -> int:
    return a + b
"""
    tests = """
import pytest
from implementation import add

def test_add_positive():
    assert add(2, 3) == 5

def test_add_negative():
    assert add(-1, -1) == -2
"""
    result = runner.run_tests(source_code=source, test_code=tests)
    assert result.exit_code == 0
    assert result.passed_tests == 2
    assert result.failed_tests == 0
    assert len(result.counter_examples) == 0


def test_sandbox_failing_execution_and_counterexample_extraction():
    runner = SandboxRunner()

    # Flawed implementation (returns wrong value for negative input)
    source = """
def is_even(n: int) -> bool:
    if n < 0:
        return True  # BUG!
    return n % 2 == 0
"""
    tests = """
import pytest
from implementation import is_even

def test_even_positive():
    assert is_even(4) is True

def test_odd_negative():
    assert is_even(-3) is False
"""
    result = runner.run_tests(source_code=source, test_code=tests)
    assert result.exit_code != 0
    assert result.failed_tests >= 1
    assert len(result.counter_examples) >= 1
    ce = result.counter_examples[0]
    assert "test_odd_negative" in ce.function_name
    assert ce.traceback is not None
