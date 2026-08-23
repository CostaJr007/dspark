"""
Unit tests for the isolated Subprocess Sandbox Runner.
"""

import unittest
from dspark.sandbox.runner import SandboxRunner


class TestSandbox(unittest.TestCase):
    def test_sandbox_passing_execution(self):
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
        self.assertEqual(result.exit_code, 0)
        self.assertEqual(result.passed_tests, 2)
        self.assertEqual(result.failed_tests, 0)
        self.assertEqual(len(result.counter_examples), 0)

    def test_sandbox_failing_execution_and_counterexample_extraction(self):
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
        self.assertNotEqual(result.exit_code, 0)
        self.assertGreaterEqual(result.failed_tests, 1)
        self.assertGreaterEqual(len(result.counter_examples), 1)
        ce = result.counter_examples[0]
        self.assertIn("test_odd_negative", ce.function_name)
        self.assertIsNotNone(ce.traceback)


if __name__ == "__main__":
    unittest.main()
