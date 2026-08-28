"""
FastMCP Server exposing DSpark Dual-Engine CEGAR verification tools.
Compatible with Antigravity, Cursor, Claude Desktop, Windsurf, and Roo Code.
"""

from __future__ import annotations

import asyncio
import json
import logging
from typing import Any, Dict, List, Optional

try:
    from mcp.server.fastmcp import FastMCP
except ImportError:
    try:
        from mcp.server.mcpserver import MCPServer as FastMCP  # mcp 2.x compat
    except ImportError:
        FastMCP = None  # type: ignore[assignment]

from ..compiler.parser import infer_contracts_from_ast
from ..engines.curator import CuratorEngine
from ..engines.refiner import RefinerEngine
from ..pipeline.cegar import CEGARPipeline
from ..state import CounterExample, IOContract

logger = logging.getLogger("dspark.mcp")


def create_mcp_server() -> FastMCP:
    """Creates and configures the DSpark FastMCP Server instance."""
    mcp = FastMCP("dspark-dual-engine")

    @mcp.tool()
    async def dspark_audit(
        code: str,
        contracts_json: Optional[str] = None,
    ) -> str:
        """
        Formally audits Python code against I/O contracts using adversarial tests in an isolated sandbox.
        Returns a structured JSON report with OS-level verdict (APPROVED/REJECTED) and counterexamples.
        """
        contracts: List[IOContract] = []
        if contracts_json:
            try:
                raw_data = json.loads(contracts_json)
                if isinstance(raw_data, list):
                    contracts = [IOContract(**item) for item in raw_data if isinstance(item, dict)]
            except Exception:
                contracts = infer_contracts_from_ast(code)
        else:
            contracts = infer_contracts_from_ast(code)

        curator = CuratorEngine()
        result = await curator.audit_and_verify(source_code=code, contracts=contracts)

        response = {
            "verdict": result.verdict.value,
            "score": result.score,
            "summary": result.summary,
            "counter_examples": [ce.model_dump() for ce in result.counter_examples],
            "passed_tests": result.sandbox_result.passed_tests if result.sandbox_result else 0,
            "failed_tests": result.sandbox_result.failed_tests if result.sandbox_result else 0,
            "execution_duration_sec": result.sandbox_result.duration_seconds if result.sandbox_result else 0.0,
            "generated_tests": result.generated_tests,
        }
        return json.dumps(response, indent=2)

    @mcp.tool()
    async def dspark_refine(
        code: str,
        counter_examples_json: str,
    ) -> str:
        """
        Applies a surgical CEGAR patch to code given deterministic counterexamples from the sandbox.
        """
        counter_examples: List[CounterExample] = []
        try:
            raw_data = json.loads(counter_examples_json)
            if isinstance(raw_data, list):
                counter_examples = [CounterExample(**item) for item in raw_data if isinstance(item, dict)]
            elif isinstance(raw_data, dict):
                counter_examples = [CounterExample(**raw_data)]
        except Exception as e:
            logger.error(f"Failed to parse counterexamples: {e}")

        refiner = RefinerEngine()
        refined_code = await refiner.refine_code(
            source_code=code,
            counter_examples=counter_examples,
        )
        return refined_code

    @mcp.tool()
    async def dspark_generate_contracts(code: str) -> str:
        """
        Parses source code AST and extracts formal Design-by-Contract (DbC) specifications.
        """
        contracts = infer_contracts_from_ast(code)
        return json.dumps([c.model_dump() for c in contracts], indent=2)

    @mcp.tool()
    async def dspark_run_cegar(
        spec: str,
        initial_code: Optional[str] = None,
    ) -> str:
        """
        Runs the full end-to-end Dual-Engine CEGAR loop: Creator -> Compiler -> Curator -> Sandbox -> Refiner.
        """
        pipeline = CEGARPipeline()
        final_state = await pipeline.execute(user_spec=spec, initial_code=initial_code)

        result = {
            "task_id": final_state.task_id,
            "verdict": final_state.verdict.value,
            "iterations": final_state.iteration,
            "final_code": final_state.current_draft,
            "contracts": [c.model_dump() for c in final_state.contracts],
            "counter_examples": [ce.model_dump() for ce in final_state.counter_examples],
            "history": final_state.history,
        }
        return json.dumps(result, indent=2)

    return mcp


def run_mcp_server():
    """Runs the FastMCP server over standard stdio."""
    mcp = create_mcp_server()
    mcp.run()


if __name__ == "__main__":
    run_mcp_server()
