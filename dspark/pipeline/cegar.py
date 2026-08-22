"""
CEGAR Dual-Engine State Graph Pipeline.
Coordinates Creator -> Compiler -> Curator -> Sandbox -> Refiner loop.
"""

from __future__ import annotations

import logging
from typing import Optional

from ..compiler.parser import infer_contracts_from_ast
from ..compiler.test_harness import ContractCompiler
from ..config import config
from ..engines.creator import CreatorEngine
from ..engines.curator import CuratorEngine
from ..engines.refiner import RefinerEngine
from ..state import DualEngineState, VerdictEnum

logger = logging.getLogger("dspark.pipeline.cegar")


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
    ):
        self.creator = creator or CreatorEngine()
        self.curator = curator or CuratorEngine()
        self.refiner = refiner or RefinerEngine()
        self.max_iterations = max_iterations or config.max_iterations

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

        # Step 2: Compile Contract Harness
        state.test_harness_code = ContractCompiler.compile_to_pytest(
            source_code=state.current_draft,
            contracts=state.contracts,
        )

        # Step 3: CEGAR Verification & Refinement Loop
        while not state.is_terminal():
            logger.info(f"Phase 2/3 (Iter {state.iteration + 1}/{state.max_iterations}): Running Curator & Sandbox...")
            
            audit_result = await self.curator.audit_and_verify(
                source_code=state.current_draft,
                contracts=state.contracts,
            )
            state.curator_test_code = audit_result.generated_tests
            state.counter_examples = audit_result.counter_examples

            # Record step in history
            state.history.append({
                "iteration": state.iteration,
                "verdict": audit_result.verdict.value,
                "score": audit_result.score,
                "summary": audit_result.summary,
                "counter_examples_count": len(audit_result.counter_examples),
            })

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

            # Step 4: Refine based on concrete counterexamples
            logger.info("Phase 4: Refining implementation with counterexample guidance...")
            refined_code = await self.refiner.refine_code(
                source_code=state.current_draft,
                counter_examples=state.counter_examples,
                contracts=state.contracts,
            )
            state.current_draft = refined_code

        return state


async def run_cegar_pipeline(user_spec: str, initial_code: Optional[str] = None) -> DualEngineState:
    """Convenience entry point to run CEGAR pipeline."""
    pipeline = CEGARPipeline()
    return await pipeline.execute(user_spec=user_spec, initial_code=initial_code)
