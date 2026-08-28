#!/usr/bin/env python3
"""CEGAR improvements A/B benchmark (offline, deterministic, zero API cost).

Runs the REAL CEGARPipeline against synthetic engines with known ground truth
(fixable / unfixable task classes) in baseline vs improved configurations and
reports exactly what each improvement buys:

  B1  memory (KDA delta rule)  : iteration savings on oscillation
  B2  VOC stagnation           : iteration savings on score plateaus
  B3  repeated evaluation K    : score variance reduction (MAE vs truth)

Verdict parity with the baseline is asserted: improvements must save work
without changing outcomes.

Usage:
  python bench/compare_cegar_improvements.py
"""
from __future__ import annotations

import random
import statistics
import sys

from dspark.config import config as global_config
from dspark.engines.creator import CreatorEngine
from dspark.engines.curator import CuratorEngine
from dspark.engines.refiner import RefinerEngine
from dspark.memory import AgentDeltaMemory
from dspark.pipeline.cegar import CEGARPipeline
from dspark.state import AuditResult, CounterExample, IOContract, VerdictEnum

import asyncio
import logging

logging.getLogger("dspark.pipeline.cegar").setLevel(logging.ERROR)

MAX_ITERATIONS = 8
N_FIXABLE = 10
N_SAME_CE = 10
N_DRIFTING = 10


class FixedCreator(CreatorEngine):
    async def generate_draft_and_contracts(self, user_spec: str, language: str = "python"):
        return (
            "def f(x):\n    return x\n",
            [IOContract(function_name="f", preconditions=["isinstance(x, int)"], postconditions=["result >= 0"])],
        )


class SyntheticCurator(CuratorEngine):
    """Task-class-driven verifier: fixable tasks eventually approve; the rest never do."""

    def __init__(self, kind: str, fix_round: int, noise_seed: int = 0):
        super().__init__()
        self.kind = kind
        self.fix_round = fix_round
        self.calls = 0
        self.rng = random.Random(noise_seed)
        self.score_noise = 0.0

    async def audit_and_verify(self, source_code: str, contracts: list):
        self.calls += 1
        noise = self.rng.uniform(-self.score_noise, self.score_noise) if self.score_noise else 0.0

        if self.kind == "fixable":
            if self.calls <= self.fix_round:
                score = max(0, min(100, int(round(40 + 30 * (self.calls - 1) + noise))))
                return AuditResult(
                    verdict=VerdictEnum.REJECTED,
                    score=score,
                    summary="fails",
                    contracts=contracts,
                    # NEW counterexample every round until fixed (memory must not converge)
                    counter_examples=[CounterExample(function_name="f", input_data={"x": -self.calls})],
                )
            return AuditResult(verdict=VerdictEnum.APPROVED, score=100, summary="ok", contracts=contracts)

        if self.kind == "same_ce":
            # Same counterexample forever: oscillation -> memory delta converges.
            score = int(round(30 + noise))
            return AuditResult(
                verdict=VerdictEnum.REJECTED,
                score=score,
                summary="fails identically",
                contracts=contracts,
                counter_examples=[CounterExample(function_name="f", input_data={"x": -1})],
            )

        # drifting: new counterexample every round, score plateau -> VOC stagnation.
        score = int(round(30 + noise))
        return AuditResult(
            verdict=VerdictEnum.REJECTED,
            score=score,
            summary="fails differently",
            contracts=contracts,
            counter_examples=[CounterExample(function_name="f", input_data={"x": -self.calls})],
        )


class SyntheticRefiner(RefinerEngine):
    """Fixes only after seeing enough DISTINCT counterexamples (new information)."""

    def __init__(self, kind: str, fix_round: int):
        super().__init__()
        self.kind = kind
        self.fix_round = fix_round
        self.seen: set = set()

    async def refine_code(self, source_code: str, counter_examples: list, contracts=None):
        if self.kind != "fixable":
            return source_code + "\n# stuck\n"
        for ce in counter_examples:
            self.seen.add((ce.function_name, str(ce.input_data)))
        if len(self.seen) >= self.fix_round:
            return "def f(x):\n    return -x if x < 0 else x\n"
        return source_code + "\n# patch attempt\n"


def run_task(kind: str, fix_round: int, memory_on: bool, voc_on: bool) -> dict:
    global_config.voc_stagnation_threshold = 0.1 if voc_on else -1.0
    global_config.memory_enabled = memory_on
    curator = SyntheticCurator(kind, fix_round)
    pipeline = CEGARPipeline(
        creator=FixedCreator(),
        curator=curator,
        refiner=SyntheticRefiner(kind, fix_round),
        max_iterations=MAX_ITERATIONS,
        memory=AgentDeltaMemory() if memory_on else None,
    )
    state = asyncio.run(pipeline.execute(user_spec=f"{kind}-{fix_round}"))
    return {
        "kind": kind,
        "verdict": state.verdict,
        "iterations": len(state.history),
        "memory_stable": state.memory_stable,
        "voc_stagnated": state.voc_stagnated,
        "curator_calls": curator.calls,
    }


def experiment_b1_b2(memory_on: bool, voc_on: bool) -> dict:
    results = []
    for i in range(N_FIXABLE):
        results.append(run_task("fixable", fix_round=(i % 3) + 1, memory_on=memory_on, voc_on=voc_on))
    for _ in range(N_SAME_CE):
        results.append(run_task("same_ce", fix_round=0, memory_on=memory_on, voc_on=voc_on))
    for _ in range(N_DRIFTING):
        results.append(run_task("drifting", fix_round=0, memory_on=memory_on, voc_on=voc_on))
    return results


def experiment_b3_k_variance():
    """Repeated evaluation K reduces score variance (MAE vs truth = 60)."""
    true_score = 60.0
    rng = random.Random(1234)

    class NoisyCurator(CuratorEngine):
        def __init__(self):
            super().__init__()
            self.calls = 0

        async def audit_and_verify(self, source_code: str, contracts: list):
            self.calls += 1
            noise = rng.uniform(-15.0, 15.0)
            return AuditResult(
                verdict=VerdictEnum.REJECTED,
                score=max(0, min(100, int(round(true_score + noise)))),
                summary="noisy",
                contracts=contracts,
                counter_examples=[CounterExample(function_name="f", input_data={"x": self.calls})],
            )

    async def sample_scores(k: int, n_iters: int) -> list:
        global_config.curator_repetitions = k
        global_config.memory_enabled = False
        global_config.voc_stagnation_threshold = -1.0  # disable VOC for this experiment
        curator = NoisyCurator()
        pipeline = CEGARPipeline(
            creator=FixedCreator(),
            curator=curator,
            refiner=SyntheticRefiner("drifting", 0),
            max_iterations=n_iters,
            memory=None,
        )
        state = await pipeline.execute(user_spec="noise")
        return [h["score"] for h in state.history], curator.calls

    k1_scores, k1_calls = asyncio.run(sample_scores(1, 6))
    k3_scores, k3_calls = asyncio.run(sample_scores(3, 6))

    mae = lambda scores: statistics.mean(abs(s - true_score) for s in scores)
    return {
        "k1_mae": mae(k1_scores),
        "k3_mae": mae(k3_scores),
        "k1_calls": k1_calls,
        "k3_calls": k3_calls,
        "k1_scores": k1_scores,
        "k3_scores": k3_scores,
    }


def summarize(results: list) -> dict:
    total_iters = sum(r["iterations"] for r in results)
    approved_by_kind = {}
    for r in results:
        approved_by_kind.setdefault(r["kind"], set()).add(r["verdict"] == VerdictEnum.APPROVED)
    return {
        "total_iterations": total_iters,
        "avg_iterations": total_iters / len(results),
        "memory_stops": sum(r["memory_stable"] for r in results),
        "voc_stops": sum(r["voc_stagnated"] for r in results),
        "total_calls": sum(r["curator_calls"] for r in results),
        "approved_by_kind": approved_by_kind,
    }


def main():
    saved = {
        "memory_enabled": global_config.memory_enabled,
        "voc_stagnation_threshold": global_config.voc_stagnation_threshold,
        "curator_repetitions": global_config.curator_repetitions,
    }
    try:
        return _main()
    finally:
        global_config.memory_enabled = saved["memory_enabled"]
        global_config.voc_stagnation_threshold = saved["voc_stagnation_threshold"]
        global_config.curator_repetitions = saved["curator_repetitions"]


def _main():
    global_config.memory_enabled = False  # baseline disables memory (explicit None fallback)
    baseline = summarize(experiment_b1_b2(memory_on=False, voc_on=False))

    global_config.memory_enabled = True
    improved = summarize(experiment_b1_b2(memory_on=True, voc_on=True))

    # Outcome parity: improvements must not change pass/fail outcomes
    # (MAX_ITER_REACHED in the baseline and REJECTED by early stop are both "fail").
    baseline_verdicts = {k: baseline["approved_by_kind"][k] for k in baseline["approved_by_kind"]}
    improved_verdicts = {k: improved["approved_by_kind"][k] for k in improved["approved_by_kind"]}
    assert baseline_verdicts == improved_verdicts, (baseline_verdicts, improved_verdicts)

    iters_saved = baseline["total_iterations"] - improved["total_iterations"]
    calls_saved = baseline["total_calls"] - improved["total_calls"]
    pct = iters_saved / baseline["total_iterations"] * 100

    k = experiment_b3_k_variance()

    print("=" * 72)
    print("B1+B2  CEGAR loop: memory (KDA) + VOC stagnation")
    print("       (30 synthetic tasks, max_iterations=8, verdict parity asserted)")
    print(f"  baseline (no early stops): {baseline['total_iterations']} iterations, {baseline['total_calls']} curator calls")
    print(f"  improved                  : {improved['total_iterations']} iterations, {improved['total_calls']} curator calls")
    print(f"  -> {iters_saved} iterations saved ({pct:.0f}% work reduction), {calls_saved} curator calls saved")
    print(f"  -> early stops: {improved['memory_stops']} memory-stable (delta<eps), {improved['voc_stops']} VOC stagnation")
    print(f"  -> pass/fail outcome parity: PASSED")
    print("=" * 72)
    print("B3  Repeated evaluation K (score variance reduction, truth = 60)")
    print(f"  K=1: MAE {k['k1_mae']:.2f}  ({k['k1_calls']} calls)   scores={k['k1_scores']}")
    print(f"  K=3: MAE {k['k3_mae']:.2f}  ({k['k3_calls']} calls)   scores={k['k3_scores']}")
    print(f"  -> score error reduced by {(1 - k['k3_mae'] / k['k1_mae']) * 100:.0f}% at 3x the calls (variance/quality trade-off knob)")
    print("=" * 72)

    assert iters_saved > 0, "improvements must save iterations"
    assert calls_saved > 0, "improvements must save curator calls"
    assert improved["memory_stops"] > 0, "memory-stable early stop must fire on oscillation tasks"
    assert improved["voc_stops"] > 0, "VOC stagnation must fire on drifting tasks"
    assert k["k3_mae"] < k["k1_mae"], "repeated evaluation must reduce score error"

    print("All improvement claims verified offline.")

    return 0


if __name__ == "__main__":
    sys.exit(main())
