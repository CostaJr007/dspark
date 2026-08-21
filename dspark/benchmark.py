"""
DSpark AI Code Generation Benchmark Suite.
Integrates the Official OpenAI HumanEval (164 problems) & EvalPlus dataset.
Measures Pass@1, edge-case resilience, and empirical accuracy delta: Baseline vs DSpark Dual-Engine.
"""

from dataclasses import dataclass, field
import gzip
import io
import json
import os
import re
import subprocess
import sys
import tempfile
import time
import urllib.request
from typing import Callable, Dict, List, Optional

from .client import DeepSeekClient
from .curator import DeepSeekCurator


OPENAI_HUMANEVAL_URL = "https://github.com/openai/human-eval/raw/master/data/HumanEval.jsonl.gz"


@dataclass
class HumanEvalTask:
    task_id: str
    prompt: str
    entry_point: str
    canonical_solution: str
    test: str


@dataclass
class TaskEvaluationResult:
    problem_id: str
    title: str
    baseline_passed: bool
    dspark_passed: bool
    baseline_time_ms: float
    dspark_time_ms: float
    curator_score: int
    contra_examples_detected: int


@dataclass
class BenchmarkReport:
    dataset_name: str
    total_problems: int
    baseline_passed_count: int
    dspark_passed_count: int
    baseline_pass_rate: float
    dspark_pass_rate: float
    accuracy_delta: float
    results: List[TaskEvaluationResult] = field(default_factory=list)


def get_dataset_cache_dir() -> str:
    home = os.path.expanduser("~")
    cache_dir = os.path.join(home, ".dspark", "datasets")
    os.makedirs(cache_dir, exist_ok=True)
    return cache_dir


def load_official_humaneval() -> List[HumanEvalTask]:
    """
    Downloads and caches the official OpenAI HumanEval dataset (164 tasks).
    """
    cache_dir = get_dataset_cache_dir()
    cache_file = os.path.join(cache_dir, "HumanEval.jsonl")

    if not os.path.exists(cache_file):
        req = urllib.request.Request(
            OPENAI_HUMANEVAL_URL,
            headers={"User-Agent": "DSpark-Benchmark/0.1.0"}
        )
        with urllib.request.urlopen(req, timeout=30) as resp:
            compressed_data = resp.read()

        buf = io.BytesIO(compressed_data)
        with gzip.GzipFile(fileobj=buf) as gz_file:
            raw_lines = gz_file.read().decode("utf-8")

        with open(cache_file, "w", encoding="utf-8") as f:
            f.write(raw_lines)

    tasks: List[HumanEvalTask] = []
    with open(cache_file, "r", encoding="utf-8") as f:
        for line in f:
            if line.strip():
                item = json.loads(line)
                tasks.append(
                    HumanEvalTask(
                        task_id=item["task_id"],
                        prompt=item["prompt"],
                        entry_point=item["entry_point"],
                        canonical_solution=item.get("canonical_solution", ""),
                        test=item["test"],
                    )
                )

    return tasks


class DSparkBenchmarkRunner:
    """
    Executes automated benchmark evaluation and generates comparative analytics.
    Supports configurable Generator (e.g. OpenAI gpt-4o-mini) and Curator (e.g. DeepSeek v4-flash).
    """

    def __init__(
        self,
        generator_model: str = "gpt-4o-mini",
        curator_model: str = "deepseek-v4-flash",
    ):
        from .client import create_model_client
        self.generator_name = generator_model
        self.curator_name = curator_model
        self.generator_client = create_model_client(generator_model)
        self.curator = DeepSeekCurator(model=curator_model)

    def _execute_humaneval_in_sandbox(self, code: str, entry_point: str, test_code: str) -> bool:
        """Executes candidate code with the official OpenAI check(entry_point) harness."""
        combined_script = f"""
{code}

# --- OFFICIAL OPENAI TEST HARNESS ---
{test_code}

try:
    check({entry_point})
    print("ALL_TESTS_PASSED_OFFICIAL")
except Exception as e:
    print(f"FAILED: {{e}}")
    sys.exit(1)
"""
        with tempfile.NamedTemporaryFile(suffix=".py", mode="w", encoding="utf-8", delete=False) as f:
            f.write(combined_script)
            temp_path = f.name

        try:
            res = subprocess.run(
                [sys.executable, temp_path],
                capture_output=True,
                text=True,
                timeout=6,
            )
            return res.returncode == 0 and "ALL_TESTS_PASSED_OFFICIAL" in res.stdout
        except Exception:
            return False
        finally:
            if os.path.exists(temp_path):
                try:
                    os.remove(temp_path)
                except Exception:
                    pass

    def run_official_humaneval_benchmark(
        self,
        limit: Optional[int] = 10,
        start_idx: int = 0,
        progress_callback: Optional[Callable[[str], None]] = None,
    ) -> BenchmarkReport:
        """
        Runs the official OpenAI HumanEval benchmark comparing Baseline vs DSpark Dual-Engine.
        """
        all_tasks = load_official_humaneval()
        end_idx = start_idx + limit if limit else len(all_tasks)
        tasks = all_tasks[start_idx:end_idx]

        results: List[TaskEvaluationResult] = []

        for idx, task in enumerate(tasks, 1):
            task_name = f"{task.task_id} ({task.entry_point})"
            if progress_callback:
                progress_callback(f"[{idx}/{len(tasks)}] Evaluating {task_name}...")

            # 1. Baseline Run (Fast Weak Model Generation, e.g. gpt-4o-mini)
            t0 = time.time()
            prompt = (
                f"Complete the following Python function following its docstring strictly:\n\n"
                f"{task.prompt}\n\n"
                f"Return only the complete Python code implementing this function."
            )
            try:
                raw_baseline = self.generator_client.complete(prompt, temperature=0.2)
                m = re.search(r"```(?:python)?\n(.*?)```", raw_baseline, re.DOTALL)
                baseline_code = m.group(1).strip() if m else raw_baseline.strip()
            except Exception:
                baseline_code = ""

            baseline_time = (time.time() - t0) * 1000
            baseline_passed = self._execute_humaneval_in_sandbox(baseline_code, task.entry_point, task.test)

            # 2. DSpark Dual-Engine Run (DeepSeek Curator Audit & Surgical Refinement)
            t1 = time.time()
            try:
                audit = self.curator.audit(
                    code=baseline_code,
                    specification=task.prompt,
                    language="python",
                )
                if audit.is_approved and audit.refined_code is None:
                    dspark_code = baseline_code
                else:
                    feedback_items = audit.critical_issues + [
                        f"Counter-example input `{ce.failing_input}` fails: expected `{ce.expected_behavior}`"
                        for ce in audit.counter_examples
                    ]
                    refine = self.curator.refine(
                        code=baseline_code,
                        specification=task.prompt,
                        feedback="\n".join(feedback_items) if feedback_items else "Ensure 100% boundary safety.",
                        language="python",
                    )
                    dspark_code = refine.refined_code
                curator_score = audit.score
                contra_count = len(audit.counter_examples) + len(audit.critical_issues)
            except Exception:
                dspark_code = baseline_code
                curator_score = 50
                contra_count = 0

            dspark_time = (time.time() - t1) * 1000
            dspark_passed = self._execute_humaneval_in_sandbox(dspark_code, task.entry_point, task.test)

            results.append(
                TaskEvaluationResult(
                    problem_id=task.task_id,
                    title=task.entry_point,
                    baseline_passed=baseline_passed,
                    dspark_passed=dspark_passed,
                    baseline_time_ms=baseline_time,
                    dspark_time_ms=dspark_time,
                    curator_score=curator_score,
                    contra_examples_detected=contra_count,
                )
            )

        total = len(results)
        base_pass = sum(1 for r in results if r.baseline_passed)
        dspark_pass = sum(1 for r in results if r.dspark_passed)

        base_rate = (base_pass / total) * 100 if total > 0 else 0.0
        dspark_rate = (dspark_pass / total) * 100 if total > 0 else 0.0
        delta = dspark_rate - base_rate

        return BenchmarkReport(
            dataset_name="OpenAI HumanEval (Official)",
            total_problems=total,
            baseline_passed_count=base_pass,
            dspark_passed_count=dspark_pass,
            baseline_pass_rate=base_rate,
            dspark_pass_rate=dspark_rate,
            accuracy_delta=delta,
            results=results,
        )
