"""
End-to-End CEGAR Pipeline test with simulated Creator, Curator and Refiner.
"""

import asyncio
import unittest
from dspark.pipeline.cegar import CEGARPipeline
from dspark.engines.creator import CreatorEngine
from dspark.engines.curator import CuratorEngine
from dspark.engines.refiner import RefinerEngine
from dspark.memory import AgentDeltaMemory
from dspark.state import IOContract, VerdictEnum


class MockCreator(CreatorEngine):
    async def generate_draft_and_contracts(self, user_spec: str, language: str = "python"):
        # Returns a buggy initial draft
        buggy_code = """
def abs_val(x: int) -> int:
    return x  # Buggy for x < 0
"""
        contracts = [
            IOContract(
                function_name="abs_val",
                preconditions=["isinstance(x, int)"],
                postconditions=["result >= 0"],
            )
        ]
        return buggy_code, contracts


class MockCurator(CuratorEngine):
    async def audit_and_verify(self, source_code: str, contracts: list[IOContract]):
        # Epistemic check: Runs sandbox with adversarial tests
        test_suite = """
import pytest
from implementation import abs_val

def test_positive():
    assert abs_val(5) == 5

def test_negative_adversarial():
    assert abs_val(-5) == 5
"""
        sandbox_res = self.sandbox.run_tests(source_code=source_code, test_code=test_suite)
        if sandbox_res.exit_code == 0:
            from dspark.state import AuditResult
            return AuditResult(
                verdict=VerdictEnum.APPROVED,
                score=100,
                summary="All adversarial tests passed.",
                contracts=contracts,
                sandbox_result=sandbox_res,
            )
        else:
            from dspark.state import AuditResult
            return AuditResult(
                verdict=VerdictEnum.REJECTED,
                score=50,
                summary="Adversarial falsification discovered bugs.",
                contracts=contracts,
                counter_examples=sandbox_res.counter_examples,
                sandbox_result=sandbox_res,
            )


class MockRefiner(RefinerEngine):
    async def refine_code(self, source_code: str, counter_examples: list, contracts=None):
        # Patches the bug based on counterexamples
        return """
def abs_val(x: int) -> int:
    return -x if x < 0 else x
"""


class TestCegarMock(unittest.TestCase):
    def test_cegar_pipeline_end_to_end_mock(self):
        async def _runner():
            pipeline = CEGARPipeline(
                creator=MockCreator(),
                curator=MockCurator(),
                refiner=MockRefiner(),
                max_iterations=3,
                memory=AgentDeltaMemory(),
            )

            final_state = await pipeline.execute(
                user_spec="Implement absolute value function that guarantees non-negative output"
            )

            # Must converge to APPROVED after 1 refinement pass
            self.assertEqual(final_state.verdict, VerdictEnum.APPROVED)
            self.assertIn("return -x if x < 0 else x", final_state.current_draft or "")
            self.assertGreaterEqual(len(final_state.history), 2)

            # AgentDeltaMemory must have recorded invariant + decision writes
            self.assertIn("channels", final_state.memory_stats)
            self.assertGreaterEqual(final_state.memory_stats["writes"], 1)

        asyncio.run(_runner())


if __name__ == "__main__":
    unittest.main()
