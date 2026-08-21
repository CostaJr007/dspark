"""
Command Line Interface for DSpark.
"""

import argparse
import json
import os
import sys
from typing import Optional

from .client import DeepSeekClient
from .curator import DeepSeekCurator
from .mcp_server import run_mcp_server
from .pipeline import DSparkPipeline


def _read_file_or_string(val: str) -> str:
    """If val is an existing file path, read its content. Otherwise return val."""
    if os.path.exists(val):
        with open(val, "r", encoding="utf-8") as f:
            return f.read()
    return val


def main():
    parser = argparse.ArgumentParser(
        prog="dspark",
        description="DSpark: Dual-LLM Speculative Code Generation & Deep Reasoning Arbitration Engine",
    )
    subparsers = parser.add_subparsers(dest="command", help="Available subcommands")

    # Command: test-connection
    test_p = subparsers.add_parser("test-connection", help="Test DeepSeek API key and endpoint")
    test_p.add_argument("--model", type=str, default=None, help="Custom model ID")

    # Command: audit
    audit_p = subparsers.add_parser("audit", help="Audit code against specifications using DeepSeek Reasoner")
    audit_p.add_argument("file", type=str, help="Path to code file or inline code string")
    audit_p.add_argument("--spec", "-s", type=str, required=True, help="Specification string or path to requirements file")
    audit_p.add_argument("--lang", "-l", type=str, default=None, help="Programming language")
    audit_p.add_argument("--json", "-j", action="store_true", help="Output raw JSON analysis")

    # Command: refine
    refine_p = subparsers.add_parser("refine", help="Refine code using DeepSeek to ensure zero edge case flaws")
    refine_p.add_argument("file", type=str, help="Path to code file or inline code string")
    refine_p.add_argument("--spec", "-s", type=str, required=True, help="Specification string or path to requirements file")
    refine_p.add_argument("--in-place", "-i", action="store_true", help="Overwrite file in-place with refined code")
    refine_p.add_argument("--out", "-o", type=str, default=None, help="Output destination file")
    refine_p.add_argument("--lang", "-l", type=str, default=None, help="Programming language")

    # Command: arbitrate
    arbitrate_p = subparsers.add_parser("arbitrate", help="Arbitrate between 2 or more candidate files")
    arbitrate_p.add_argument("files", nargs="+", help="Paths to candidate code files")
    arbitrate_p.add_argument("--spec", "-s", type=str, required=True, help="Specification string or path to requirements file")
    arbitrate_p.add_argument("--lang", "-l", type=str, default=None, help="Programming language")

    # Command: run (Pipeline: Generator -> Curator -> Final Code)
    run_p = subparsers.add_parser("run", help="Run full dual-model pipeline (Generate -> Curate -> Output)")
    run_p.add_argument("prompt", type=str, help="Feature request or algorithm specification")
    run_p.add_argument("--draft", "-d", type=str, default=None, help="Optional draft code file to curate instead of generating")
    run_p.add_argument("--lang", "-l", type=str, default=None, help="Programming language")
    run_p.add_argument("--out", "-o", type=str, default=None, help="Output destination file")

    # Command: mcp
    subparsers.add_parser("mcp", help="Run DSpark as a Model Context Protocol (MCP) server")

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        sys.exit(1)

    try:
        if args.command == "test-connection":
            client = DeepSeekClient(default_model=args.model)
            print(f"Connecting to DeepSeek API at: {client.base_url} (model: {client.default_model})...")
            res = client.complete("Ping. Respond with 'DSpark Online'.", temperature=0.0)
            print(f"Success! Response: {res.strip()}")

        elif args.command == "audit":
            curator = DeepSeekCurator()
            code = _read_file_or_string(args.file)
            spec = _read_file_or_string(args.spec)
            result = curator.audit(code=code, specification=spec, language=args.lang)

            if args.json:
                print(result.raw_response)
            else:
                verdict_color = "\033[92m" if result.is_approved else "\033[93m" if result.verdict.value == "NEEDS_REVISION" else "\033[91m"
                reset_color = "\033[0m"
                print(f"\n{verdict_color}=== DSPARK AUDIT VERDICT: {result.verdict.value} (Score: {result.score}/100) ==={reset_color}\n")
                print(f"Summary: {result.summary}\n")
                
                if result.critical_issues:
                    print("Critical Issues:")
                    for issue in result.critical_issues:
                        print(f"  [!] {issue}")
                    print()

                if result.edge_cases:
                    print("Edge Case Analysis:")
                    for ec in result.edge_cases:
                        status = "✓ Handled" if ec.handled_properly else f"✗ Flaw ({ec.risk_level} Risk): {ec.remedy}"
                        print(f"  - {ec.case}: {status}")
                    print()

                if result.complexity:
                    print(f"Complexity: Time {result.complexity.get('time', 'N/A')}, Space {result.complexity.get('space', 'N/A')}\n")

                if result.suggested_improvements:
                    print("Suggested Improvements:")
                    for imp in result.suggested_improvements:
                        print(f"  * {imp}")
                    print()

        elif args.command == "refine":
            curator = DeepSeekCurator()
            code = _read_file_or_string(args.file)
            spec = _read_file_or_string(args.spec)
            result = curator.refine(code=code, specification=spec, language=args.lang)

            if args.in_place and os.path.exists(args.file):
                with open(args.file, "w", encoding="utf-8") as f:
                    f.write(result.refined_code)
                print(f"Refined code written in-place to {args.file}")
            elif args.out:
                with open(args.out, "w", encoding="utf-8") as f:
                    f.write(result.refined_code)
                print(f"Refined code written to {args.out}")
            else:
                print(result.refined_code)

        elif args.command == "arbitrate":
            curator = DeepSeekCurator()
            candidates = [_read_file_or_string(f) for f in args.files]
            spec = _read_file_or_string(args.spec)
            result = curator.arbitrate(candidates=candidates, specification=spec, language=args.lang)

            print(f"\n=== DSPARK ARBITRATION RESULT ===")
            print(f"Winner: Candidate #{result.winner_index}")
            print(f"Rationale: {result.rationale}\n")
            print("Optimal Synthesized Code:")
            print(result.synthesized_code)

        elif args.command == "run":
            pipeline = DSparkPipeline()
            draft = _read_file_or_string(args.draft) if args.draft else None
            res = pipeline.run(specification=args.prompt, draft_code=draft, language=args.lang)

            print(f"\n=== DSPARK PIPELINE COMPLETED ===")
            print(f"Audit Verdict: {res.audit_result.verdict.value} (Score: {res.audit_result.score}/100)")
            print(f"Refined by Curator: {res.refined}\n")
            
            if args.out:
                with open(args.out, "w", encoding="utf-8") as f:
                    f.write(res.final_code)
                print(f"Final verified code written to {args.out}")
            else:
                print("Final Verified Code:")
                print(res.final_code)

        elif args.command == "mcp":
            run_mcp_server()

    except Exception as e:
        sys.stderr.write(f"Error: {e}\n")
        sys.exit(1)


if __name__ == "__main__":
    main()
