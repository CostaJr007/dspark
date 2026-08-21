"""
Model Context Protocol (MCP) server implementation for DSpark.
Enables native integration with Antigravity, Claude Desktop, Cursor, and any MCP client.
"""

import json
import sys
import traceback
from typing import Any, Dict

from .curator import DeepSeekCurator


def handle_tool_call(curator: DeepSeekCurator, name: str, arguments: Dict[str, Any]) -> Dict[str, Any]:
    if name == "dspark_audit_code":
        code = arguments.get("code", "")
        spec = arguments.get("specification", "")
        lang = arguments.get("language")
        result = curator.audit(code=code, specification=spec, language=lang)
        return {
            "verdict": result.verdict.value,
            "score": result.score,
            "summary": result.summary,
            "io_contract_analysis": result.io_contract_analysis,
            "edge_cases": [
                {
                    "case": ec.case,
                    "risk_level": ec.risk_level,
                    "handled": ec.handled_properly,
                    "remedy": ec.remedy,
                }
                for ec in result.edge_cases
            ],
            "complexity": result.complexity,
            "critical_issues": result.critical_issues,
            "suggested_improvements": result.suggested_improvements,
            "refined_code": result.refined_code,
        }

    elif name == "dspark_refine_code":
        code = arguments.get("code", "")
        spec = arguments.get("specification", "")
        feedback = arguments.get("feedback")
        lang = arguments.get("language")
        res = curator.refine(code=code, specification=spec, feedback=feedback, language=lang)
        return {
            "refined_code": res.refined_code,
            "summary_of_changes": res.summary_of_changes,
        }

    elif name == "dspark_arbitrate":
        candidates = arguments.get("candidates", [])
        spec = arguments.get("specification", "")
        lang = arguments.get("language")
        res = curator.arbitrate(candidates=candidates, specification=spec, language=lang)
        return {
            "winner_index": res.winner_index,
            "rationale": res.rationale,
            "comparison_matrix": res.comparison_matrix,
            "synthesized_code": res.synthesized_code,
        }

    else:
        raise ValueError(f"Unknown tool name: {name}")


def run_mcp_server():
    """Runs a standard stdio JSON-RPC MCP server."""
    curator = DeepSeekCurator()

    TOOLS = [
        {
            "name": "dspark_audit_code",
            "description": "Perform deep reasoning analysis and I/O contract arbitration on candidate code using DeepSeek.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "code": {"type": "string", "description": "The candidate code to audit."},
                    "specification": {"type": "string", "description": "The requirements, task prompt, or I/O contract specification."},
                    "language": {"type": "string", "description": "Programming language (e.g. python, typescript, rust, c++)."}
                },
                "required": ["code", "specification"]
            }
        },
        {
            "name": "dspark_refine_code",
            "description": "Refine code using DeepSeek to ensure 100% production readiness, strict typing, and edge case coverage.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "code": {"type": "string", "description": "Draft code to optimize/fix."},
                    "specification": {"type": "string", "description": "Requirements or goals."},
                    "feedback": {"type": "string", "description": "Specific audit feedback or issues to fix."},
                    "language": {"type": "string", "description": "Programming language."}
                },
                "required": ["code", "specification"]
            }
        },
        {
            "name": "dspark_arbitrate",
            "description": "Arbitrate between two or more alternative code implementations and synthesize the optimal solution.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "candidates": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of candidate implementations to compare."
                    },
                    "specification": {"type": "string", "description": "The requirements to evaluate against."},
                    "language": {"type": "string", "description": "Programming language."}
                },
                "required": ["candidates", "specification"]
            }
        }
    ]

    for line in sys.stdin:
        if not line.strip():
            continue
        try:
            req = json.loads(line)
            req_id = req.get("id")
            method = req.get("method")

            if method == "initialize":
                resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {"tools": {}},
                        "serverInfo": {"name": "dspark-mcp", "version": "0.1.0"},
                    },
                }
            elif method == "tools/list":
                resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {"tools": TOOLS},
                }
            elif method == "tools/call":
                params = req.get("params", {})
                tool_name = params.get("name")
                args = params.get("arguments", {})
                result_payload = handle_tool_call(curator, tool_name, args)
                resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [
                            {"type": "text", "text": json.dumps(result_payload, indent=2)}
                        ]
                    },
                }
            else:
                resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {"code": -32601, "message": f"Method {method} not supported"},
                }

            sys.stdout.write(json.dumps(resp) + "\n")
            sys.stdout.flush()

        except Exception as e:
            sys.stderr.write(f"Error handling MCP request: {traceback.format_exc()}\n")
            sys.stderr.flush()
