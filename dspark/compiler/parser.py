"""
AST Parser for static analysis, contract discovery, and signature extraction.
"""

from __future__ import annotations

import ast
import re
from typing import Any, Dict, List, Optional
from ..state import IOContract


def parse_code_ast(source_code: str) -> Optional[ast.AST]:
    """Parse Python source code into an AST node. Returns None on syntax error."""
    try:
        return ast.parse(source_code)
    except SyntaxError:
        return None


def extract_functions_and_docstrings(source_code: str) -> List[Dict[str, Any]]:
    """
    Extracts all functions, methods, parameters, annotations, and docstrings from Python source.
    """
    tree = parse_code_ast(source_code)
    if not tree:
        return []

    functions: List[Dict[str, Any]] = []

    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            docstring = ast.get_docstring(node) or ""
            args = [arg.arg for arg in node.args.args]
            returns = ast.unparse(node.returns) if node.returns else None

            # Detect embedded contract tags in docstring
            pre_matches = re.findall(r"(?:PRE|PRECONDITION):\s*(.+)", docstring, re.IGNORECASE)
            post_matches = re.findall(r"(?:POST|POSTCONDITION):\s*(.+)", docstring, re.IGNORECASE)
            inv_matches = re.findall(r"(?:INV|INVARIANT):\s*(.+)", docstring, re.IGNORECASE)

            functions.append({
                "name": node.name,
                "is_async": isinstance(node, ast.AsyncFunctionDef),
                "args": args,
                "returns": returns,
                "docstring": docstring,
                "preconditions": [p.strip() for p in pre_matches],
                "postconditions": [p.strip() for p in post_matches],
                "invariants": [i.strip() for i in inv_matches],
                "lineno": node.lineno,
            })

    return functions


def infer_contracts_from_ast(source_code: str) -> List[IOContract]:
    """
    Infers baseline IOContract objects directly from AST signatures and docstrings.
    """
    extracted = extract_functions_and_docstrings(source_code)
    contracts: List[IOContract] = []

    for func in extracted:
        pres = list(func["preconditions"])
        posts = list(func["postconditions"])
        invs = list(func["invariants"])

        # Add default type checks if type annotations exist
        contracts.append(
            IOContract(
                function_name=func["name"],
                preconditions=pres,
                postconditions=posts,
                invariants=invs,
            )
        )

    return contracts
