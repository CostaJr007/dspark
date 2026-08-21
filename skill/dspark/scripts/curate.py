#!/usr/bin/env python3
"""
Lightweight helper script for AGY agents to audit or refine a specific file.

Prefers the native `dspark` Rust binary; falls back to the optional Python SDK.
"""

import argparse
import shutil
import subprocess
import sys
import os

script_dir = os.path.dirname(os.path.abspath(__file__))
project_root = os.path.abspath(os.path.join(script_dir, "..", "..", ".."))
if project_root not in sys.path:
    sys.path.insert(0, project_root)


def run_rust_cli(args) -> bool:
    binary = shutil.which("dspark")
    if not binary:
        release = os.path.join(project_root, "target", "release", "dspark")
        debug = os.path.join(project_root, "target", "debug", "dspark")
        for candidate in (release, release + ".exe", debug, debug + ".exe"):
            if os.path.isfile(candidate):
                binary = candidate
                break
    if not binary:
        return False

    if args.refine:
        cmd = [binary, "refine", args.file, "--spec", args.spec, "--in-place"]
    else:
        cmd = [binary, "audit", args.file, "--spec", args.spec]
    raise SystemExit(subprocess.call(cmd))


def main():
    parser = argparse.ArgumentParser(description="Quick DeepSeek curation for AGY")
    parser.add_argument("file", help="File to audit or refine")
    parser.add_argument("--spec", required=True, help="Requirements or specification")
    parser.add_argument("--refine", action="store_true", help="Apply refinement in place")
    args = parser.parse_args()

    if not os.path.exists(args.file):
        print(f"Error: file '{args.file}' not found.")
        sys.exit(1)

    try:
        run_rust_cli(args)
    except SystemExit as e:
        if e.code is not None:
            raise

    from dspark.curator import DeepSeekCurator

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
