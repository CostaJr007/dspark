"""
Unit tests for DSpark State Models and Circuit Breaker logic.
"""

import unittest
from dspark.state import (
    CounterExample,
    DualEngineState,
    IOContract,
    VerdictEnum,
    AuditResult,
    SandboxExecutionResult,
)


class TestDSparkState(unittest.TestCase):
    def test_io_contract_creation(self):
        contract = IOContract(
            function_name="divide",
            preconditions=["b != 0", "isinstance(a, (int, float))"],
            postconditions=["result == a / b"],
            invariants=["isinstance(result, float)"],
        )
        self.assertEqual(contract.function_name, "divide")
        self.assertEqual(len(contract.preconditions), 2)
        self.assertEqual(len(contract.postconditions), 1)

    def test_counter_example_creation(self):
        ce = CounterExample(
            function_name="divide",
            input_data={"a": 10, "b": 0},
            expected_behavior="Raises ZeroDivisionError or ValueError",
            actual_behavior="Unhandled ZeroDivisionError",
            failing_assert_code="assert divide(10, 0) == 'error'",
            traceback="ZeroDivisionError: division by zero",
        )
        self.assertEqual(ce.function_name, "divide")
        self.assertEqual(ce.input_data["b"], 0)
        self.assertIsNotNone(ce.id)

    def test_dual_engine_state_circuit_breaker(self):
        state = DualEngineState(
            user_spec="Build a safe division function",
            creator_model="gemini/gemini-2.5-flash",
            curator_model="deepseek/deepseek-chat",
            refiner_model="deepseek/deepseek-chat",
            max_iterations=2,
        )

        self.assertEqual(state.verdict, VerdictEnum.PENDING)
        self.assertFalse(state.is_terminal())
        self.assertEqual(state.iteration, 0)

        # Iteration 1
        state.increment_iteration()
        self.assertEqual(state.iteration, 1)
        self.assertFalse(state.is_terminal())

        # Iteration 2 (Hits max_iterations)
        state.increment_iteration()
        self.assertEqual(state.iteration, 2)
        self.assertEqual(state.verdict, VerdictEnum.MAX_ITER_REACHED)
        self.assertTrue(state.is_terminal())

    def test_dual_engine_state_approval(self):
        state = DualEngineState(
            user_spec="Build a safe division function",
            creator_model="gemini/gemini-2.5-flash",
            curator_model="deepseek/deepseek-chat",
            refiner_model="deepseek/deepseek-chat",
            max_iterations=3,
        )
        state.verdict = VerdictEnum.APPROVED
        self.assertTrue(state.is_terminal())


if __name__ == "__main__":
    unittest.main()
