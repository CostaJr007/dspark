"""
Tests for the verification-scaling improvements ported from the research papers:
- repeated evaluation K (LLM-as-a-Verifier, arXiv:2607.05391 Section 4.2)
- criteria decomposition aggregation (Section 4.3)
- VOC progress signal + stagnation stop (Section 6)
- Spearman rank correlation helper
"""

import asyncio
import unittest

from dspark.pipeline.cegar import CEGARPipeline, _spearman_rank_correlation
from dspark.engines.creator import CreatorEngine
from dspark.engines.curator import CuratorEngine
from dspark.engines.refiner import RefinerEngine
from dspark.memory import AgentDeltaMemory
from dspark.state import AuditResult, CounterExample, DualEngineState, IOContract, VerdictEnum


class TestSpearman(unittest.TestCase):

    def test_constant_scores_are_flat(self):
        self.assertEqual(_spearman_rank_correlation([0, 1, 2], [30.0, 30.0, 30.0]), 0.0)

    def test_increasing_scores_are_positive(self):
        self.assertAlmostEqual(_spearman_rank_correlation([0, 1, 2], [10.0, 50.0, 90.0]), 1.0)

    def test_decreasing_scores_are_negative(self):
        self.assertAlmostEqual(_spearman_rank_correlation([0, 1, 2], [90.0, 50.0, 10.0]), -1.0)

    def test_ties_are_averaged(self):
        corr = _spearman_rank_correlation([0, 1, 2, 3], [10.0, 10.0, 50.0, 90.0])
        self.assertTrue(0.0 < corr < 1.0)


class MockCreator(CreatorEngine):
    async def generate_draft_and_contracts(self, user_spec: str, language: str = "python"):
        return (
            "def f(x):\n    return x\n",
            [IOContract(function_name="f", preconditions=["isinstance(x, int)"], postconditions=["result >= 0"])],
        )


class NeverFixesRefiner(RefinerEngine):
    async def refine_code(self, source_code: str, counter_examples: list, contracts=None):
        return source_code + "\n# patched\n"


class CountingApproveCurator(CuratorEngine):
    """Approves every audit with a score that rises with the call index."""

    def __init__(self):
        super().__init__()
        self.calls = 0

    async def audit_and_verify(self, source_code: str, contracts: list):
        self.calls += 1
        return AuditResult(
            verdict=VerdictEnum.APPROVED,
            score=60 + self.calls * 10,
            summary="ok",
            contracts=contracts,
        )


class MixedVerdictCurator(CuratorEngine):
    """Alternates APPROVED/REJECTED with per-criterion scores."""

    def __init__(self):
        super().__init__()
        self.calls = 0

    async def audit_and_verify(self, source_code: str, contracts: list):
        self.calls += 1
        if self.calls % 2 == 1:
            return AuditResult(
                verdict=VerdictEnum.APPROVED,
                score=90,
                summary="ok",
                contracts=contracts,
                criteria_scores={"specification": 90, "output": 90, "errors": 90},
            )
        return AuditResult(
            verdict=VerdictEnum.REJECTED,
            score=40,
            summary="no",
            contracts=contracts,
            counter_examples=[CounterExample(function_name="f", input_data={"x": self.calls})],
            criteria_scores={"specification": 40, "output": 40, "errors": 40},
        )


class DriftingRejectCurator(CuratorEngine):
    """Always rejects with a NEW counterexample each call and a constant score."""

    def __init__(self):
        super().__init__()
        self.calls = 0

    async def audit_and_verify(self, source_code: str, contracts: list):
        self.calls += 1
        return AuditResult(
            verdict=VerdictEnum.REJECTED,
            score=30,
            summary="fails",
            contracts=contracts,
            counter_examples=[CounterExample(function_name="f", input_data={"x": -self.calls})],
        )


class TestVerificationScaling(unittest.TestCase):

    def test_repeated_evaluation_averages_scores(self):
        from dspark.config import config as global_config
        original = global_config.curator_repetitions
        try:
            global_config.curator_repetitions = 3
            async def _runner():
                curator = CountingApproveCurator()
                pipeline = CEGARPipeline(
                    creator=MockCreator(),
                    curator=curator,
                    refiner=NeverFixesRefiner(),
                    max_iterations=3,
                    memory=AgentDeltaMemory(),
                )
                state = await pipeline.execute(user_spec="Implement f")
                self.assertEqual(state.verdict, VerdictEnum.APPROVED)
                # K=3 independent audits per iteration; the loop ends after 1 iteration.
                self.assertEqual(curator.calls, 3)
                # Mean of (70, 80, 90)
                self.assertEqual(state.history[-1]["score"], 80)
            asyncio.run(_runner())
        finally:
            global_config.curator_repetitions = original

    def test_repeated_evaluation_verdict_is_conservative_and_merges(self):
        from dspark.config import config as global_config
        original = global_config.curator_repetitions
        try:
            global_config.curator_repetitions = 3
            async def _runner():
                curator = MixedVerdictCurator()
                pipeline = CEGARPipeline(
                    creator=MockCreator(),
                    curator=curator,
                    refiner=NeverFixesRefiner(),
                    max_iterations=3,
                    memory=AgentDeltaMemory(),
                )
                state = DualEngineState(
                    user_spec="x",
                    language="python",
                    creator_model="a",
                    curator_model="b",
                    refiner_model="c",
                    max_iterations=3,
                )
                result = await pipeline._run_audits(state)
                # A, R, A -> conservative REJECTED; score = round((90+40+90)/3)
                self.assertEqual(result.verdict, VerdictEnum.REJECTED)
                self.assertEqual(result.score, 73)
                # Counterexamples deduped across repetitions (only the rejected one)
                self.assertEqual(len(result.counter_examples), 1)
                # Criteria scores averaged: round((90+40+90)/3) = 73
                self.assertEqual(result.criteria_scores["specification"], 73)
                self.assertEqual(result.criteria_scores["errors"], 73)
            asyncio.run(_runner())
        finally:
            global_config.curator_repetitions = original

    def test_voc_stagnation_stops_drifting_rejections(self):
        async def _runner():
            pipeline = CEGARPipeline(
                creator=MockCreator(),
                curator=DriftingRejectCurator(),
                refiner=NeverFixesRefiner(),
                max_iterations=8,
                memory=AgentDeltaMemory(),
            )
            state = await pipeline.execute(user_spec="Implement f")
            self.assertEqual(state.verdict, VerdictEnum.REJECTED)
            self.assertTrue(state.voc_stagnated, "flat scores must trigger the VOC stagnation stop")
            self.assertFalse(state.memory_stable, "new counterexamples each round must not converge memory")
            self.assertEqual(len(state.history), 3, "stagnation fires at the minimum history length")
            self.assertIsNotNone(state.voc)
            self.assertLess(state.voc, 0.1)
            self.assertIn("VOC stagnation", state.error_message)
        asyncio.run(_runner())


if __name__ == "__main__":
    unittest.main()
