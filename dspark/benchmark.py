"""
DSpark AI Code Generation Benchmark Suite.
Measures Pass@1, edge-case resilience, and accuracy gain: Single-LLM Baseline vs DSpark Dual-Engine.
Inspired by HumanEval+, SWE-bench, and LiveCodeBench.
"""

from dataclasses import dataclass, field
import json
import os
import subprocess
import sys
import tempfile
import time
from typing import Callable, Dict, List, Optional

from .client import DeepSeekClient
from .curator import DeepSeekCurator
from .pipeline import DSparkPipeline


@dataclass
class BenchmarkProblem:
    id: str
    title: str
    specification: str
    canonical_test_code: str
    edge_cases_description: str


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
    total_problems: int
    baseline_passed_count: int
    dspark_passed_count: int
    baseline_pass_rate: float
    dspark_pass_rate: float
    accuracy_delta: float
    results: List[TaskEvaluationResult] = field(default_factory=list)


# Curated benchmark dataset of challenging edge-case problems
BENCHMARK_DATASET: List[BenchmarkProblem] = [
    BenchmarkProblem(
        id="DSP-01",
        title="Safe Binary Search with Boundary & Type Safety",
        specification="""Write a function `binary_search(arr: list[int], target: int) -> int`
        Requirements:
        - Return index of target if found in sorted list, else return -1.
        - Must handle empty list `[]` returning -1 without IndexError.
        - Must handle target smaller than min or greater than max.
        - Must achieve O(log N) time complexity.
        """,
        canonical_test_code="""
assert binary_search([], 5) == -1, "Failed on empty list"
assert binary_search([1, 3, 5, 7, 9], 5) == 2, "Failed on target in middle"
assert binary_search([1, 3, 5, 7, 9], 1) == 0, "Failed on target at start"
assert binary_search([1, 3, 5, 7, 9], 9) == 4, "Failed on target at end"
assert binary_search([1, 3, 5, 7, 9], 0) == -1, "Failed on target smaller than min"
assert binary_search([1, 3, 5, 7, 9], 10) == -1, "Failed on target greater than max"
assert binary_search([2], 2) == 0, "Failed on single-element match"
assert binary_search([2], 3) == -1, "Failed on single-element non-match"
print("ALL TESTS PASSED")
""",
        edge_cases_description="Empty array, single element, search boundaries (0 and N-1), off-by-one errors.",
    ),
    BenchmarkProblem(
        id="DSP-02",
        title="Sliding Window Maximum (O(N) Deque Invariant)",
        specification="""Write a function `max_sliding_window(nums: list[int], k: int) -> list[int]`
        Requirements:
        - Return maximum value in every sliding window of size k moving from left to right.
        - If `nums` is empty or `k <= 0`, return `[]`.
        - If `k >= len(nums)`, return `[max(nums)]`.
        - Must run in strictly O(N) time using deque/monotonic queue.
        """,
        canonical_test_code="""
assert max_sliding_window([], 3) == [], "Failed on empty list"
assert max_sliding_window([1, 3, -1, -3, 5, 3, 6, 7], 3) == [3, 3, 5, 5, 6, 7], "Failed standard window"
assert max_sliding_window([1], 1) == [1], "Failed single element"
assert max_sliding_window([1, -1], 1) == [1, -1], "Failed k=1"
assert max_sliding_window([9, 11], 5) == [11], "Failed k > len"
assert max_sliding_window([7, 2, 4], 2) == [7, 4], "Failed decreasing window"
print("ALL TESTS PASSED")
""",
        edge_cases_description="k > len(nums), k <= 0, empty list, negative values, monotonic decreasing inputs.",
    ),
    BenchmarkProblem(
        id="DSP-03",
        title="LRU Cache with O(1) Operations & Capacity Invariants",
        specification="""Implement class `LRUCache`:
        - `__init__(self, capacity: int)`: initialize with positive capacity. Handle capacity <= 0 gracefully (treat as 0 capacity).
        - `get(self, key: int) -> int`: return value if present and mark as recently used, else return -1.
        - `put(self, key: int, value: int) -> None`: update or insert key-value. If size exceeds capacity, evict least recently used.
        - Both get and put must strictly run in O(1) average time complexity.
        """,
        canonical_test_code="""
cache = LRUCache(2)
cache.put(1, 1)
cache.put(2, 2)
assert cache.get(1) == 1, "Failed to get existing key"
cache.put(3, 3) # evicts key 2
assert cache.get(2) == -1, "Failed to evict least recently used key 2"
cache.put(4, 4) # evicts key 1
assert cache.get(1) == -1, "Failed to evict key 1"
assert cache.get(3) == 3, "Key 3 should be present"
assert cache.get(4) == 4, "Key 4 should be present"
# Test zero capacity edge case
empty_cache = LRUCache(0)
empty_cache.put(1, 100)
assert empty_cache.get(1) == -1, "Capacity 0 should store nothing"
print("ALL TESTS PASSED")
""",
        edge_cases_description="Zero capacity, cache eviction order updates on get(), overwriting existing keys without increasing size.",
    ),
]


class DSparkBenchmarkRunner:
    """
    Executes automated benchmark evaluation and generates comparative analytics.
    """

    def __init__(self, curator: Optional[DeepSeekCurator] = None):
        self.curator = curator or DeepSeekCurator()
        self.client = self.curator.client

    def _execute_in_sandbox(self, code: str, test_code: str) -> bool:
        """Executes candidate code + test assertions in a dedicated isolated Python process."""
        combined_script = f"{code}\n\n# --- CANONICAL TEST HARNESS ---\n{test_code}"
        with tempfile.NamedTemporaryFile(suffix=".py", mode="w", encoding="utf-8", delete=False) as f:
            f.write(combined_script)
            temp_path = f.name

        try:
            res = subprocess.run(
                [sys.executable, temp_path],
                capture_output=True,
                text=True,
                timeout=5,
            )
            return res.returncode == 0 and "ALL TESTS PASSED" in res.stdout
        except Exception:
            return False
        finally:
            if os.path.exists(temp_path):
                try:
                    os.remove(temp_path)
                except Exception:
                    pass

    def run_benchmark(
        self,
        problems: Optional[List[BenchmarkProblem]] = None,
        progress_callback: Optional[Callable[[str], None]] = None,
    ) -> BenchmarkReport:
        target_problems = problems or BENCHMARK_DATASET
        results: List[TaskEvaluationResult] = []

        for prob in target_problems:
            if progress_callback:
                progress_callback(f"Evaluating {prob.id}: {prob.title}...")

            # 1. Generate Baseline (Fast One-Shot Prompt)
            t0 = time.time()
            baseline_prompt = f"Implement in Python:\n{prob.specification}\nReturn only the Python code in a markdown code block."
            try:
                baseline_code_raw = self.client.complete(baseline_prompt, temperature=0.7)
                # Extract code block
                import re
                m = re.search(r"```(?:python)?\n(.*?)```", baseline_code_raw, re.DOTALL)
                baseline_code = m.group(1).strip() if m else baseline_code_raw.strip()
            except Exception:
                baseline_code = ""
            baseline_time = (time.time() - t0) * 1000
            baseline_passed = self._execute_in_sandbox(baseline_code, prob.canonical_test_code)

            # 2. Generate DSpark Dual-Engine (Audit + Refine Guided by Counter-Examples)
            t1 = time.time()
            try:
                audit = self.curator.audit(code=baseline_code, specification=prob.specification, language="python")
                if audit.is_approved and audit.refined_code is None:
                    dspark_code = baseline_code
                else:
                    refine = self.curator.refine(
                        code=baseline_code,
                        specification=prob.specification,
                        feedback="\n".join(audit.critical_issues + [ec.case for ec in audit.edge_cases]),
                        language="python",
                    )
                    dspark_code = refine.refined_code
                curator_score = audit.score
                contra_count = len(audit.critical_issues) + len([e for e in audit.edge_cases if not e.handled_properly])
            except Exception:
                dspark_code = baseline_code
                curator_score = 50
                contra_count = 0
            dspark_time = (time.time() - t1) * 1000
            dspark_passed = self._execute_in_sandbox(dspark_code, prob.canonical_test_code)

            results.append(
                TaskEvaluationResult(
                    problem_id=prob.id,
                    title=prob.title,
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
            total_problems=total,
            baseline_passed_count=base_pass,
            dspark_passed_count=dspark_pass,
            baseline_pass_rate=base_rate,
            dspark_pass_rate=dspark_rate,
            accuracy_delta=delta,
            results=results,
        )
