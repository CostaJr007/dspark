"""
CEGAR Dual-Engine State Graph Pipeline.
Coordinates Creator -> Compiler -> Curator -> Sandbox -> Refiner loop.

Memory (AgentDeltaMemory): the loop carries a KDA-derived fixed-size memory
(see dspark.memory) whose delta rule corrects knowledge instead of appending it.
  - invariants (contracts)   -> channel="invariant" (alpha ~ 1, never forgotten)
  - task verdicts            -> channel="decision"
  - counterexample outcomes  -> channel="transient" (beta=0.6)
A converged write (delta < eps) means the loop no longer learns for that
counterexample: the delta->0 theorem triggers a memory-stable early stop,
avoiding wasteful refinement passes on an oscillation.
"""

from __future__ import annotations

import asyncio
import hashlib
import json
import logging
import math
from typing import Dict, List, Optional, Sequence

from ..compiler.parser import infer_contracts_from_ast
from ..compiler.test_harness import ContractCompiler
from ..config import config
from ..engines.creator import CreatorEngine
from ..engines.curator import CuratorEngine
from ..engines.refiner import RefinerEngine
from ..memory import AgentDeltaMemory
from ..state import AuditResult, CounterExample, DualEngineState, VerdictEnum

logger = logging.getLogger("dspark.pipeline.cegar")


def _spearman_rank_correlation(xs: Sequence[float], ys: Sequence[float]) -> float:
    """Spearman rank correlation with average ties (Value-Order Correlation).

    Quantifies whether the verifier score tracks task progress across iterations
    (LLM-as-a-Verifier, arXiv:2607.05391 Section 6): monotonically rising scores
    give VOC -> 1; a flat or falling loop gives VOC <= 0.
    """
    n = len(xs)
    if n < 2:
        return 0.0

    def rank_with_ties(values: Sequence[float]) -> List[float]:
        order = sorted(range(n), key=lambda i: values[i])
        ranks = [0.0] * n
        i = 0
        while i < n:
            j = i
            while j + 1 < n and values[order[j + 1]] == values[order[i]]:
                j += 1
            avg = (i + j) / 2.0 + 1.0
            for idx in order[i : j + 1]:
                ranks[idx] = avg
            i = j + 1
        return ranks

    rx = rank_with_ties(xs)
    ry = rank_with_ties(ys)
    mx = sum(rx) / n
    my = sum(ry) / n
    num = sum((x - mx) * (y - my) for x, y in zip(rx, ry))
    dx = math.sqrt(sum((x - mx) ** 2 for x in rx))
    dy = math.sqrt(sum((y - my) ** 2 for y in ry))
    if dx == 0.0 or dy == 0.0:
        return 0.0
    return num / (dx * dy)


class CEGARPipeline:
    """
    Asynchronous CEGAR (Counterexample-Guided Abstraction Refinement) Pipeline.
    """

    def __init__(
        self,
        creator: Optional[CreatorEngine] = None,
        curator: Optional[CuratorEngine] = None,
        refiner: Optional[RefinerEngine] = None,
        max_iterations: Optional[int] = None,
        memory: Optional[AgentDeltaMemory] = None,
    ):
        self.creator = creator or CreatorEngine()
        self.curator = curator or CuratorEngine()
        self.refiner = refiner or RefinerEngine()
        self.max_iterations = max_iterations or config.max_iterations
        self.memory = (
            memory
            if memory is not None
            else (
                AgentDeltaMemory(
                    dim=config.memory_dim,
                    eps=config.memory_eps,
                )
                if config.memory_enabled
                else None
            )
        )

    @staticmethod
    def _task_outcome_key(state: DualEngineState) -> str:
        # Digest split into 4 token-chunks so the discriminating part dominates the
        # key embedding: distinct tasks share only {task, outcome} (2/6 tokens,
        # cosine ~0.33 < key_similarity 0.45) while the same task binds at 1.0.
        digest = hashlib.sha256(state.user_spec.encode("utf-8")).hexdigest()[:16]
        return (
            f"task:{digest[:4]}:{digest[4:8]}:{digest[8:12]}:{digest[12:16]}:outcome"
        )

    @staticmethod
    def _ce_key(language: str, ce: CounterExample) -> str:
        # Distinct counterexamples of the same function share only {ce, fn}
        # (2/5 tokens, cosine ~0.40 < 0.45) so they bind to separate entries;
        # the exact same counterexample binds at cosine 1.0 and converges.
        canonical = json.dumps(ce.input_data, sort_keys=True, default=str)
        digest = hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:12]
        return f"ce:{digest[:4]}:{digest[4:8]}:{digest[8:12]}:{ce.function_name}"

    def _memorize_invariants(self, state: DualEngineState) -> None:
        """Write formal contracts as invariant-class knowledge (alpha ~ 1)."""
        if self.memory is None:
            return
        for contract in state.contracts:
            props = (
                contract.preconditions
                + contract.postconditions
                + contract.invariants
            )
            if not props:
                continue
            digest = hashlib.sha256(contract.function_name.encode("utf-8")).hexdigest()[:12]
            self.memory.write(
                key=(
                    f"contract:{state.language}:{digest[:4]}:{digest[4:8]}:"
                    f"{digest[8:12]}:{contract.function_name}"
                ),
                value="\n".join(props),
                channel="invariant",
                beta=0.95,
            )

    async def _run_audits(self, state: DualEngineState) -> AuditResult:
        """Repeated evaluation (K): run independent curator audits and aggregate.

        Variance reduction per LLM-as-a-Verifier Section 4.2. The verdict is
        conservative (APPROVED only if every repetition approves); the score is the
        mean; counterexamples are unioned with dedup; criteria scores are averaged.
        """
        k = config.curator_repetitions
        if k <= 1:
            return await self.curator.audit_and_verify(
                source_code=state.current_draft,
                contracts=state.contracts,
            )

        results = await asyncio.gather(
            *[
                self.curator.audit_and_verify(
                    source_code=state.current_draft,
                    contracts=state.contracts,
                )
                for _ in range(k)
            ]
        )
        score = int(round(sum(r.score for r in results) / len(results)))
        verdict = (
            VerdictEnum.APPROVED
            if all(r.verdict == VerdictEnum.APPROVED for r in results)
            else VerdictEnum.REJECTED
        )
        seen: set = set()
        counter_examples: List[CounterExample] = []
        for r in results:
            for ce in r.counter_examples:
                ce_key = (ce.function_name, json.dumps(ce.input_data, sort_keys=True, default=str))
                if ce_key not in seen:
                    seen.add(ce_key)
                    counter_examples.append(ce)
        criteria_scores: Dict[str, int] = {}
        for name in ("specification", "output", "errors"):
            values = [r.criteria_scores[name] for r in results if name in r.criteria_scores]
            if values:
                criteria_scores[name] = int(round(sum(values) / len(values)))

        first = results[0]
        return AuditResult(
            verdict=verdict,
            score=score,
            summary=f"{first.summary} (K={k} repeated evaluations)",
            contracts=first.contracts,
            counter_examples=counter_examples,
            generated_tests=first.generated_tests,
            criteria_scores=criteria_scores,
            raw_response="\n\n".join(r.raw_response for r in results),
        )

    async def execute(
        self,
        user_spec: str,
        initial_code: Optional[str] = None,
        language: str = "python",
    ) -> DualEngineState:
        """
        Executes the complete Dual-Engine CEGAR verification cycle.
        """
        state = DualEngineState(
            user_spec=user_spec,
            language=language,
            creator_model=self.creator.model,
            curator_model=self.curator.model,
            refiner_model=self.refiner.model,
            max_iterations=self.max_iterations,
        )

        if self.memory is not None:
            self.memory.decay(1)

        # Step 1: Draft Code & Formal Contracts
        if not initial_code:
            logger.info("Phase 1: Generating draft implementation via Creator...")
            draft_code, contracts = await self.creator.generate_draft_and_contracts(
                user_spec=user_spec,
                language=language,
            )
            state.current_draft = draft_code
            state.contracts = contracts
        else:
            logger.info("Phase 1: Using provided code and inferring AST contracts...")
            state.current_draft = initial_code
            state.contracts = infer_contracts_from_ast(initial_code)

        # Memory: commit formal contracts as invariant-class knowledge (alpha ~ 1).
        self._memorize_invariants(state)

        # Step 2: Compile Contract Harness
        state.test_harness_code = ContractCompiler.compile_to_pytest(
            source_code=state.current_draft,
            contracts=state.contracts,
        )

        # Step 3: CEGAR Verification & Refinement Loop
        while not state.is_terminal():
            logger.info(f"Phase 2/3 (Iter {state.iteration + 1}/{state.max_iterations}): Running Curator & Sandbox...")

            audit_result = await self._run_audits(state)
            state.curator_test_code = audit_result.generated_tests
            state.counter_examples = audit_result.counter_examples

            # Memory: delta-rule writes for the verdict and counterexample outcomes.
            memory_converged = False
            if self.memory is not None:
                if audit_result.verdict == VerdictEnum.APPROVED:
                    self.memory.write(
                        key=self._task_outcome_key(state),
                        value=f"APPROVED score={audit_result.score}",
                        channel="decision",
                        beta=1.0,
                        label="APPROVED",
                    )
                else:
                    self.memory.write(
                        key=self._task_outcome_key(state),
                        value=f"REJECTED score={audit_result.score}",
                        channel="decision",
                        beta=0.7,
                        label="REJECTED",
                    )
                    converged_flags = []
                    for ce in audit_result.counter_examples:
                        result = self.memory.write(
                            key=self._ce_key(state.language, ce),
                            value="REJECTED",
                            channel="transient",
                            beta=0.6,
                            label="REJECTED",
                        )
                        converged_flags.append(result.converged)
                    memory_converged = bool(converged_flags) and all(converged_flags)

            # Record step in history
            state.history.append({
                "iteration": state.iteration,
                "verdict": audit_result.verdict.value,
                "score": audit_result.score,
                "summary": audit_result.summary,
                "counter_examples_count": len(audit_result.counter_examples),
                "memory_converged": memory_converged,
            })

            # VOC progress signal: does the verifier score track iteration progress?
            voc = _spearman_rank_correlation(
                [float(h["iteration"]) for h in state.history],
                [float(h["score"]) for h in state.history],
            )
            state.voc = voc
            state.history[-1]["voc"] = voc

            # Check if verified
            if audit_result.verdict == VerdictEnum.APPROVED:
                state.verdict = VerdictEnum.APPROVED
                logger.info("Verification SUCCESS: All adversarial sandbox tests passed.")
                break

            # If rejected, check circuit breaker
            state.increment_iteration()
            if state.verdict == VerdictEnum.MAX_ITER_REACHED:
                logger.warning("Circuit breaker tripped: Max iterations reached without approval.")
                break

            # Memory-stable early stop (delta -> 0): every counterexample write converged,
            # so the loop is no longer learning -- refining again would just re-oscillate.
            # `iteration >= 1` guards against an oversized eps making first writes converge.
            if memory_converged and state.iteration >= 1:
                state.memory_stable = True
                state.verdict = VerdictEnum.REJECTED
                state.error_message = (
                    "Memory-stable (delta < eps): refinement loop stopped learning; "
                    "repeated counterexamples detected, skipping redundant refinement."
                )
                logger.warning("Memory-stable early stop: %s", state.error_message)
                break

            # VOC stagnation stop (verifier progress signal): the score no longer
            # correlates with iteration progress over a sustained rejection run.
            if (
                state.voc is not None
                and len(state.history) >= config.voc_stagnation_min_points
                and state.voc < config.voc_stagnation_threshold
                and all(h["verdict"] == "REJECTED" for h in state.history)
            ):
                state.voc_stagnated = True
                state.verdict = VerdictEnum.REJECTED
                state.error_message = (
                    f"VOC stagnation: Spearman(iteration, score) = {state.voc:.3f} "
                    f"below threshold {config.voc_stagnation_threshold}; "
                    "refinement is no longer improving the verifier score."
                )
                logger.warning("VOC stagnation early stop: %s", state.error_message)
                break

            # Step 4: Refine based on concrete counterexamples
            logger.info("Phase 4: Refining implementation with counterexample guidance...")
            refined_code = await self.refiner.refine_code(
                source_code=state.current_draft,
                counter_examples=state.counter_examples,
                contracts=state.contracts,
            )
            state.current_draft = refined_code

        if self.memory is not None:
            state.memory_stats = self.memory.stats()

        return state


async def run_cegar_pipeline(user_spec: str, initial_code: Optional[str] = None) -> DualEngineState:
    """Convenience entry point to run CEGAR pipeline."""
    pipeline = CEGARPipeline()
    return await pipeline.execute(user_spec=user_spec, initial_code=initial_code)
