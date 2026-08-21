#!/usr/bin/env python3
"""
Lightweight helper script for AGY agents to audit or refine a specific file.
"""

import argparse
import sys
import os

# Add parent directories to path if running standalone
script_dir = os.path.dirname(os.path.abspath(__file__))
project_root = os.path.abspath(os.path.join(script_dir, "..", ".."))
if project_root not in sys.path:
    sys.path.insert(0, project_root)

from dspark.curator import DeepSeekCurator


def main():
    parser = argparse.ArgumentParser(description="Quick DeepSeek curation for AGY")
    parser.add_argument("file", help="File to audit or refine")
    parser.add_argument("--spec", required=True, help="Requirements or specification")
    parser.add_argument("--refine", action="store_true", help="Apply refinement in place")
    args = parser.parse_args()

    if not os.path.exists(args.file):
        print(f"Error: file '{args.file}' not found.")
        sys.exit(1)

    with open(args.file, "r", encoding="utf-8") as f:
        code = f.read()

    curator = DeepSeekCurator()

    if args.refine:
        res = curator.refine(code=code, specification=args.spec)
        with open(args.file, "w", encoding="utf-8") as f:
            f.write(res.refined_code)
        print(f"Successfully refined '{args.file}'.")
    else:
        audit = curator.audit(code=code, specification=args.spec)
        print(f"Verdict: {audit.verdict.value} (Score: {audit.score}/100)")
        print(f"Summary: {audit.summary}")
        if audit.critical_issues:
            print("Issues:")
            for issue in audit.critical_issues:
                print(f" - {issue}")


if __name__ == "__main__":
    main()
