"""
Unit tests for AgentDeltaMemory (KDA-derived agent memory) and its CEGAR integration.
"""

import asyncio
import unittest

from dspark.memory import AgentDeltaMemory, MemoryRead, MemoryWrite
from dspark.pipeline.cegar import CEGARPipeline
from dspark.state import AuditResult, CounterExample, IOContract, VerdictEnum
from dspark.engines.creator import CreatorEngine
from dspark.engines.curator import CuratorEngine
from dspark.engines.refiner import RefinerEngine


class TestAgentDeltaMemory(unittest.TestCase):

    def test_write_read_roundtrip(self):
        mem = AgentDeltaMemory()
        result = mem.write(
            "contract:python:abs_val", "result >= 0", channel="invariant", beta=0.95
        )
        self.assertIsInstance(result, MemoryWrite)
        self.assertTrue(result.updated)
        self.assertFalse(result.converged)

        read = mem.lookup("contract:python:abs_val", channel="invariant")
        self.assertIsInstance(read, MemoryRead)
        self.assertIsNotNone(read.value)
        self.assertEqual(read.entries_hit, 1)

    def test_repeated_write_converges_delta_rule(self):
        mem = AgentDeltaMemory()
        key = "ce:python:abs_val:abc123"
        first = mem.write(key, "REJECTED", channel="transient", beta=0.6)
        self.assertFalse(first.converged, "first write is an insert, never converged")

        second = mem.write(key, "REJECTED", channel="transient", beta=0.6)
        self.assertTrue(second.converged, "delta rule must converge for identical outcomes")
        self.assertLess(second.delta_norm, mem.eps)

        third = mem.write(key, "REJECTED", channel="transient", beta=0.6)
        self.assertTrue(third.converged)
        self.assertEqual(mem.stats()["converged_writes"], 2)

    def test_per_channel_decay_invariant_survives_transient_evicted(self):
        mem = AgentDeltaMemory()
        mem.write("inv", "a = 1", channel="invariant", beta=1.0)
        mem.write("trn", "b = 2", channel="transient", beta=0.6)

        for _ in range(200):
            mem.decay(1)

        self.assertIsNotNone(
            mem.lookup("inv", channel="invariant").value,
            "invariants must survive decay (alpha ~ 1)",
        )
        self.assertIsNone(
            mem.lookup("trn", channel="transient").value,
            "transient entries must be evicted after many time-steps (alpha << 1)",
        )

    def test_key_bound_updates_are_surgical(self):
        mem = AgentDeltaMemory()
        mem.write("k1", "value_one", channel="decision", beta=0.5)
        before = mem.lookup("k1", channel="decision").value

        mem.write("k2_completely_different", "value_two", channel="decision", beta=0.5)
        after = mem.lookup("k1", channel="decision").value

        diff = sum(abs(a - b) for a, b in zip(before, after))
        self.assertLess(
            diff, 1e-9,
            "unrelated write must not mutate a key-bound entry (rank-1 update)",
        )

    def test_outcome_projection_distinguishes_approved_and_rejected(self):
        mem = AgentDeltaMemory()
        mem.write("t1", "APPROVED score=100", channel="decision", beta=1.0, label="APPROVED")
        mem.write("t2", "REJECTED score=40", channel="decision", beta=1.0, label="REJECTED")

        self.assertEqual(mem.predict_outcome("t1")[0], "APPROVED")
        self.assertEqual(mem.predict_outcome("t2")[0], "REJECTED")

    def test_outcome_vote_needs_majority_weight(self):
        mem = AgentDeltaMemory()
        # Conflicting labels bound to the same key void the entry vote (no outcome).
        mem.write("k", "ok", channel="decision", beta=0.5, label="APPROVED")
        mem.write("k", "nope", channel="decision", beta=0.5, label="REJECTED")
        read = mem.lookup("k", channel="decision")
        self.assertIsNone(read.outcome, "conflicting labels must not produce a verdict")

    def test_unknown_channel_raises(self):
        mem = AgentDeltaMemory()
        with self.assertRaises(KeyError):
            mem.write("k", "v", channel="nope")

    def test_distinct_counterexamples_do_not_bind_each_other(self):
        """Regression: keys of different CEs for the same function must stay
        below key_similarity (0.45), otherwise a NEW counterexample would read
        as `converged` and wrongly trigger the memory-stable early stop."""
        from dspark.pipeline.cegar import CEGARPipeline
        mem = AgentDeltaMemory()
        ce1 = CounterExample(function_name="abs_val", input_data={"x": -1})
        ce2 = CounterExample(function_name="abs_val", input_data={"x": -2})
        k1 = CEGARPipeline._ce_key("python", ce1)
        k2 = CEGARPipeline._ce_key("python", ce2)

        first = mem.write(k1, "REJECTED", channel="transient", beta=0.6)
        self.assertFalse(first.converged, "insert must not converge")

        second = mem.write(k2, "REJECTED", channel="transient", beta=0.6)
        self.assertFalse(
            second.converged,
            "a different counterexample is a fresh failure and must not read as converged",
        )

        third = mem.write(k1, "REJECTED", channel="transient", beta=0.6)
        self.assertTrue(third.converged, "the same counterexample repeated must converge")

    def test_distinct_tasks_do_not_bind_each_other(self):
        """Regression: task outcome keys for different specs must not merge."""
        from dspark.pipeline.cegar import CEGARPipeline
        from dspark.state import DualEngineState
        mem = AgentDeltaMemory()
        s1 = DualEngineState(
            user_spec="Implement abs",
            language="python",
            creator_model="a",
            curator_model="b",
            refiner_model="c",
        )
        s2 = DualEngineState(
            user_spec="Implement sort",
            language="python",
            creator_model="a",
            curator_model="b",
            refiner_model="c",
        )
        k1 = CEGARPipeline._task_outcome_key(s1)
        k2 = CEGARPipeline._task_outcome_key(s2)

        mem.write(k1, "APPROVED score=95", channel="decision", beta=1.0, label="APPROVED")
        result = mem.write(k2, "REJECTED score=30", channel="decision", beta=0.7, label="REJECTED")
        self.assertFalse(result.converged, "distinct tasks must not bind into one entry")
        self.assertEqual(mem.predict_outcome(k2)[0], "REJECTED")
        self.assertEqual(mem.predict_outcome(k1)[0], "APPROVED")

    def test_stats_shape(self):
        mem = AgentDeltaMemory()
        mem.write("k", "v", channel="decision")
        stats = mem.stats()
        self.assertIn("channels", stats)
        self.assertIn("invariant", stats["channels"])
        self.assertGreaterEqual(stats["writes"], 1)


class MockCreator(CreatorEngine):
    async def generate_draft_and_contracts(self, user_spec: str, language: str = "python"):
        return (
            "def f(x):\n    return x\n",
            [IOContract(
                function_name="f",
                preconditions=["isinstance(x, int)"],
                postconditions=["result >= 0"],
            )],
        )


class ApproveCurator(CuratorEngine):
    async def audit_and_verify(self, source_code: str, contracts: list):
        return AuditResult(
            verdict=VerdictEnum.APPROVED,
            score=95,
            summary="ok",
            contracts=contracts,
        )


class AlwaysRejectCurator(CuratorEngine):
    async def audit_and_verify(self, source_code: str, contracts: list):
        return AuditResult(
            verdict=VerdictEnum.REJECTED,
            score=30,
            summary="always fails",
            contracts=contracts,
            counter_examples=[CounterExample(function_name="f", input_data={"x": -1})],
        )


class NeverFixesRefiner(RefinerEngine):
    async def refine_code(self, source_code: str, counter_examples: list, contracts=None):
        return source_code + "\n# patched\n"


class TestMemoryCegarIntegration(unittest.TestCase):

    def test_approved_flow_records_memory_stats(self):
        async def _runner():
            memory = AgentDeltaMemory()
            pipeline = CEGARPipeline(
                creator=MockCreator(),
                curator=ApproveCurator(),
                refiner=NeverFixesRefiner(),
                max_iterations=3,
                memory=memory,
            )
            state = await pipeline.execute(user_spec="Implement f")
            self.assertEqual(state.verdict, VerdictEnum.APPROVED)
            self.assertIn("channels", state.memory_stats)
            self.assertGreaterEqual(state.memory_stats["writes"], 2)  # invariants + decision
            self.assertFalse(state.memory_stable)
        asyncio.run(_runner())

    def test_memory_stable_early_stop_on_oscillation(self):
        async def _runner():
            memory = AgentDeltaMemory()
            pipeline = CEGARPipeline(
                creator=MockCreator(),
                curator=AlwaysRejectCurator(),
                refiner=NeverFixesRefiner(),
                max_iterations=6,
                memory=memory,
            )
            state = await pipeline.execute(user_spec="Implement f")
            self.assertEqual(state.verdict, VerdictEnum.REJECTED)
            self.assertTrue(state.memory_stable, "delta->0 must trigger the stable stop")
            self.assertEqual(len(state.history), 2,
                             "early stop must fire after the first repeated counterexample")
            self.assertGreaterEqual(state.memory_stats["converged_writes"], 1)
        asyncio.run(_runner())

    def test_no_memory_when_disabled(self):
        async def _runner():
            from dspark.config import config as global_config
            original = global_config.memory_enabled
            try:
                global_config.memory_enabled = False
                pipeline = CEGARPipeline(
                    creator=MockCreator(),
                    curator=ApproveCurator(),
                    refiner=NeverFixesRefiner(),
                    max_iterations=3,
                )
                self.assertIsNone(pipeline.memory)
                state = await pipeline.execute(user_spec="Implement f")
                self.assertEqual(state.verdict, VerdictEnum.APPROVED)
                self.assertEqual(state.memory_stats, {})
            finally:
                global_config.memory_enabled = original
        asyncio.run(_runner())


if __name__ == "__main__":
    unittest.main()
