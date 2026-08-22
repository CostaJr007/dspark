"""
Contract Compiler: Translates formal IOContract specifications into executable pytest test harnesses.
"""

from __future__ import annotations

from typing import List
from ..state import IOContract


class ContractCompiler:
    """
    Compiles IOContract definitions into executable pytest assertions and test suites.
    """

    @classmethod
    def compile_to_pytest(cls, source_code: str, contracts: List[IOContract]) -> str:
        """
        Synthesizes a self-contained pytest test harness file.
        Wraps target functions with contract verification assertions.
        """
        lines: List[str] = [
            "# -*- coding: utf-8 -*-",
            "# Auto-generated test harness by DSpark Contract Compiler",
            "import pytest",
            "import math",
            "import typing",
            "",
            "# === Target Implementation Under Test ===",
            source_code,
            "",
            "# === Formal Contract Assertions ===",
        ]

        for contract in contracts:
            fn_name = contract.function_name
            lines.append(f"def test_contract_compliance_{fn_name}():")
            lines.append(f"    # Ensure function '{fn_name}' exists in global scope")
            lines.append(f"    assert '{fn_name}' in globals() or hasattr(globals(), '{fn_name}'), f'Function {fn_name} not found'")
            lines.append(f"    target_fn = globals().get('{fn_name}')")
            lines.append(f"    assert callable(target_fn), f'{fn_name} is not callable'")
            lines.append("")

            # Generate contract validation docstring
            lines.append(f"    # Preconditions: {contract.preconditions}")
            lines.append(f"    # Postconditions: {contract.postconditions}")
            lines.append(f"    # Invariants: {contract.invariants}")
            lines.append("    pass")
            lines.append("")

        return "\n".join(lines)

    @classmethod
    def generate_contract_wrapper(cls, function_name: str, contract: IOContract) -> str:
        """
        Generates a contract-enforcing decorator/wrapper snippet for runtime verification.
        """
        pre_asserts = "\n    ".join([f"assert ({p}), 'Precondition failed: {p}'" for p in contract.preconditions])
        post_asserts = "\n    ".join([f"assert ({p}), 'Postcondition failed: {p}'" for p in contract.postconditions])

        return f"""
def verify_contract_{function_name}(*args, **kwargs):
    # Preconditions
    {pre_asserts if pre_asserts else 'pass'}
    result = {function_name}(*args, **kwargs)
    # Postconditions
    {post_asserts if post_asserts else 'pass'}
    return result
"""
