"""
Isolated Subprocess Sandbox Runner for pytest and formal contract verification.
"""

from __future__ import annotations

import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time
from typing import List, Optional

from ..config import config
from ..state import CounterExample, SandboxExecutionResult


class SandboxRunner:
    """
    Executes Python source code and synthesized test suites in an isolated temporary sandbox.
    Determines verdict strictly from OS process return code and parses pytest failures.
    """

    def __init__(self, timeout_seconds: Optional[int] = None, base_dir: Optional[Path] = None):
        self.timeout = timeout_seconds or config.sandbox_timeout_seconds
        self.base_dir = base_dir or config.sandbox_temp_dir

    def run_tests(
        self,
        source_code: str,
        test_code: str,
        module_name: str = "implementation",
    ) -> SandboxExecutionResult:
        """
        Runs the test_code against source_code in an isolated sandbox.
        """
        # Ensure base directory exists
        self.base_dir.mkdir(parents=True, exist_ok=True)
        temp_dir = Path(tempfile.mkdtemp(prefix="dspark_sb_", dir=self.base_dir))

        start_time = time.time()
        try:
            # 1. Write implementation file
            src_file = temp_dir / f"{module_name}.py"
            src_file.write_text(source_code, encoding="utf-8")

            # 2. Write test file (ensure imports resolve)
            # If test code doesn't import implementation, prepend helper import
            full_test_code = test_code
            if f"import {module_name}" not in test_code and f"from {module_name}" not in test_code:
                full_test_code = f"import sys\nfrom pathlib import Path\nsys.path.insert(0, str(Path(__file__).parent))\nfrom {module_name} import *\n\n{test_code}"

            test_file = temp_dir / "test_suite.py"
            test_file.write_text(full_test_code, encoding="utf-8")

            # 3. Execute pytest via subprocess
            cmd = [
                sys.executable,
                "-m",
                "pytest",
                "-v",
                "--tb=short",
                "-rA",
                str(test_file),
            ]

            env = os.environ.copy()
            env["PYTHONPATH"] = str(temp_dir) + os.pathsep + env.get("PYTHONPATH", "")

            proc = subprocess.run(
                cmd,
                cwd=str(temp_dir),
                capture_output=True,
                text=True,
                timeout=self.timeout,
                env=env,
            )

            duration = time.time() - start_time
            counter_examples = self._parse_pytest_output(proc.stdout, proc.stderr)
            passed_count, failed_count = self._count_test_results(proc.stdout)

            return SandboxExecutionResult(
                exit_code=proc.returncode,
                stdout=proc.stdout,
                stderr=proc.stderr,
                passed_tests=passed_count,
                failed_tests=failed_count,
                duration_seconds=duration,
                timed_out=False,
                counter_examples=counter_examples,
            )

        except subprocess.TimeoutExpired as te:
            duration = time.time() - start_time
            timeout_ce = CounterExample(
                function_name="execution_timeout",
                input_data={"timeout_seconds": self.timeout},
                expected_behavior="Termination within allocated time budget",
                actual_behavior=f"Process timed out after {self.timeout}s (possible infinite loop or hang)",
                failing_assert_code="assert execution_time < timeout",
                traceback=str(te),
            )
            return SandboxExecutionResult(
                exit_code=124,
                stdout=te.stdout or "" if isinstance(te.stdout, str) else "",
                stderr=te.stderr or "" if isinstance(te.stderr, str) else "",
                passed_tests=0,
                failed_tests=1,
                duration_seconds=duration,
                timed_out=True,
                counter_examples=[timeout_ce],
            )
        except Exception as ex:
            duration = time.time() - start_time
            error_ce = CounterExample(
                function_name="sandbox_runner",
                input_data={},
                expected_behavior="Sandbox process execution",
                actual_behavior=f"Sandbox runner error: {str(ex)}",
                failing_assert_code="",
                traceback=str(ex),
            )
            return SandboxExecutionResult(
                exit_code=1,
                stdout="",
                stderr=str(ex),
                passed_tests=0,
                failed_tests=1,
                duration_seconds=duration,
                timed_out=False,
                counter_examples=[error_ce],
            )
        finally:
            # Clean up sandbox directory
            try:
                shutil.rmtree(temp_dir, ignore_errors=True)
            except Exception:
                pass

    def _count_test_results(self, stdout: str) -> tuple[int, int]:
        """Counts passed and failed tests from pytest summary."""
        passed = 0
        failed = 0
        
        passed_match = re.search(r"(\d+)\s+passed", stdout)
        if passed_match:
            passed = int(passed_match.group(1))

        failed_match = re.search(r"(\d+)\s+failed", stdout)
        if failed_match:
            failed = int(failed_match.group(1))

        error_match = re.search(r"(\d+)\s+error", stdout)
        if error_match:
            failed += int(error_match.group(1))

        return passed, failed

    def _parse_pytest_output(self, stdout: str, stderr: str) -> List[CounterExample]:
        """Parses stdout/stderr into concrete CounterExample objects."""
        counter_examples: List[CounterExample] = []

        # 1. Match FAILURES / ERRORS individual test blocks
        failure_pattern = re.compile(
            r"_{3,}\s+([^\n_]+)\s+_{3,}(.*?)(?=(?:_{3,}\s+[^\n_]+\s+_{3,})|(?:={3,}\s+)|$)",
            re.DOTALL,
        )

        for match in failure_pattern.finditer(stdout):
            test_name = match.group(1).strip()
            traceback_block = match.group(2).strip()

            # Extract failing assertion line
            failing_line = ""
            assert_match = re.search(r">\s+(assert .+)", traceback_block)
            if assert_match:
                failing_line = assert_match.group(1)
            else:
                # Look for exception line
                err_lines = [l for l in traceback_block.splitlines() if l.startswith("E   ")]
                if err_lines:
                    failing_line = err_lines[-1]

            # Extract error message
            error_msg = ""
            e_lines = [l[4:].strip() for l in traceback_block.splitlines() if l.startswith("E   ")]
            if e_lines:
                error_msg = " | ".join(e_lines)

            ce = CounterExample(
                function_name=test_name,
                input_data={"test_case": test_name},
                expected_behavior="Test assertion to pass without error",
                actual_behavior=error_msg or "Assertion failure or unhandled exception",
                failing_assert_code=failing_line,
                traceback=traceback_block,
            )
            counter_examples.append(ce)

        # 2. Fallback: Parse from short test summary info if block regex missed it
        if not counter_examples:
            for line in stdout.splitlines():
                if line.startswith("FAILED "):
                    parts = line.split(" - ", 1)
                    test_id = parts[0].replace("FAILED ", "").strip()
                    error_msg = parts[1].strip() if len(parts) > 1 else "Test failed"
                    counter_examples.append(
                        CounterExample(
                            function_name=test_id.split("::")[-1] if "::" in test_id else test_id,
                            input_data={"test_case": test_id},
                            expected_behavior="Assertion to hold true",
                            actual_behavior=error_msg,
                            failing_assert_code=error_msg,
                            traceback=line,
                        )
                    )

        return counter_examples
