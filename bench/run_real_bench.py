#!/usr/bin/env python3
"""DSpark tiered-pipeline real benchmark pilot.

Providers : gpt-4o-mini (cheap tier) + deepseek-v4-flash (flagship tier)
Tasks     : HumanEval subset (resolution) + 6 mini creation tasks (contracts)
Budget    : hard kill-switch per provider (default $0.45)

One MAIN logged run yields the tiered configs AND their counterfactuals:

  B  cheap-only       draft[0] alone                       (separate calls)
  A  flagship-only    single v4-flash attempt              (separate calls)
  C  best-of-N random E[random draft passes]               (counterfactual, free)
  V  verify-all       first sandbox-passing draft          (counterfactual, free)
  T  PPT pick         tournament winner                    (counterfactual, free)
  D  full tiered      T + escalation to v4-flash on fail   (measured)

Q1 (LLM-as-a-Verifier value)  = paired delta T vs C   (+ T vs V context)
Q2 (v4-flash added value)     = paired delta D vs T   + escalation precision

Usage:
  python bench/run_real_bench.py --smoke            # 3 tasks, ~$0.01
  python bench/run_real_bench.py --limit 50         # full pilot
"""
from __future__ import annotations

import argparse
import gzip
import io
import json
import os
import random
import re
import statistics
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

HERE = Path(__file__).parent if "__file__" in globals() else Path("bench").resolve()
RESULTS_DIR = HERE / "results"
HUMANEVAL_URL = "https://raw.githubusercontent.com/openai/human-eval/master/data/HumanEval.jsonl.gz"

# ---------------------------------------------------------------- assumptions
PRICE_PER_TOKEN = {  # EDIT ME to match your provider invoices.
    "gpt-4o-mini": {"in": 0.15e-6, "out": 0.60e-6},
    "gpt-3.5-turbo": {"in": 0.50e-6, "out": 1.50e-6},
    "gpt-3.5-turbo-0125": {"in": 0.50e-6, "out": 1.50e-6},
    "deepseek-v4-flash": {"in": 0.30e-6, "out": 0.90e-6},  # placeholder, conservative
    "deepseek-chat": {"in": 0.27e-6, "out": 1.10e-6},
    "deepseek-reasoner": {"in": 0.55e-6, "out": 2.19e-6},
}
BUDGET_CAP_USD = 0.45  # kill-switch per provider
# deepseek-v4-flash is a REASONING model: its internal thinking shares the
# completion-token budget, so the cap must leave room for it (empirically up
# to ~1.6k chars). A tight cap yields empty/truncated code intermittently.
MAX_COMPLETION_TOKENS = {"openai": 600, "deepseek": 4096}
DRAFT_TEMPERATURES = [0.2, 0.5, 0.8]
N_DRAFTS = len(DRAFT_TEMPERATURES)
K_PIVOTS_REQUESTED = 2
TIE_EPSILON = 0.05
SANDBOX_TIMEOUT_S = 12

# Global socket watchdog: every socket op is bounded so a dead connection can
# never hang the pilot (urllib's per-request timeout alone is not enough when
# the OS socket itself stalls).
import socket  # noqa: E402

socket.setdefaulttimeout(120)


def log(msg: str) -> None:
    print(msg, flush=True)


# ---------------------------------------------------------------------- LLM I/O
class Budget:
    def __init__(self) -> None:
        self.spend: dict[str, float] = {}

    def add(self, model: str, usage: dict) -> float:
        p = PRICE_PER_TOKEN[model]
        cost = usage.get("prompt_tokens", 0) * p["in"] + usage.get("completion_tokens", 0) * p["out"]
        self.spend[model] = self.spend.get(model, 0.0) + cost
        if self.spend[model] > BUDGET_CAP_USD:
            raise SystemExit(
                f"BUDGET KILL-SWITCH: {model} reached ${self.spend[model]:.4f} > ${BUDGET_CAP_USD}"
            )
        return cost

    def total(self) -> float:
        return sum(self.spend.values())


BUDGET = Budget()


def get_provider(model: str) -> str:
    return "deepseek" if ("deepseek" in model.lower() or "qwen" in model.lower()) else "openai"


def chat(model: str, messages: list[dict], temperature: float, provider: str | None = None) -> tuple[str, dict]:
    provider = provider or get_provider(model)
    if provider == "openai":
        url = "https://api.openai.com/v1/chat/completions"
        key = os.environ["OPENAI_API_KEY"]
    else:
        url = "https://api.deepseek.com/chat/completions"
        key = os.environ["DEEPSEEK_API_KEY"]
    body = json.dumps({
        "model": model,
        "messages": messages,
        "temperature": temperature,
        "max_tokens": MAX_COMPLETION_TOKENS[provider],
    }).encode()
    req = urllib.request.Request(url, data=body, headers={
        "Content-Type": "application/json",
        "Authorization": f"Bearer {key}",
    })
    for attempt in range(2):
        try:
            with urllib.request.urlopen(req, timeout=90) as resp:
                data = json.loads(resp.read())
            text = data["choices"][0]["message"]["content"] or ""
            usage = data.get("usage", {})
            BUDGET.add(model, usage)
            return text, usage
        except Exception as e:  # noqa: BLE001 - one retry, then surface
            if attempt == 1:
                raise
            log(f"    retry after error: {e}")
            time.sleep(3)
    raise RuntimeError("unreachable")


def extract_code(text: str) -> str:
    fence = re.search(r"```(?:python)?\s*\n(.*?)```", text, re.S)
    code = fence.group(1) if fence else text
    # drop any leading prose lines that are not python-ish
    lines = code.strip().splitlines()
    while lines and not (lines[0].strip().startswith(("def ", "class ", "import ", "from ", "#"))
                        or "=" in lines[0]):
        lines.pop(0)
    return "\n".join(lines)


# --------------------------------------------------------------------- sandbox
def run_contracts(full_code: str, suite: str | None, entry_point: str | None) -> dict:
    """Execute candidate against contracts in a fresh interpreter. PASS/FAIL."""
    if suite is not None:  # creation task: pytest-style suite importing solution
        program = full_code + "\n\n" + suite
    else:  # human-eval style: tests define check(candidate); invoke it
        program = full_code + "\n\n" + (suite_or_default(entry_point) or "")
    with tempfile.TemporaryDirectory() as td:
        f = Path(td) / "case.py"
        f.write_text(program, encoding="utf-8")
        try:
            proc = subprocess.run(
                [sys.executable, str(f)],
                capture_output=True, text=True, timeout=SANDBOX_TIMEOUT_S,
                cwd=td, env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
            )
            passed = proc.returncode == 0
            tail = (proc.stdout + "\n" + proc.stderr).strip()[-400:]
        except subprocess.TimeoutExpired:
            passed, tail = False, "TIMEOUT"
    return {"passed": passed, "tail": tail}


def suite_or_default(entry_point: str | None) -> str:
    # For HumanEval rows the `test` field already arrives appended by caller.
    return ""


def run_pytest_suite(candidate_code: str, suite: str) -> dict:
    """Creation tasks: write solution.py + suite runner."""
    runner = (
        "import sys, pytest, io\n"
        "sys.path.insert(0, '.')\n"
    )
    with tempfile.TemporaryDirectory() as td:
        Path(td, "solution.py").write_text(candidate_code, encoding="utf-8")
        test_file = Path(td, "test_solution.py")
        test_file.write_text("import solution\n" + suite, encoding="utf-8")
        try:
            proc = subprocess.run(
                [sys.executable, "-m", "pytest", "-xq", test_file.name],
                capture_output=True, text=True, timeout=SANDBOX_TIMEOUT_S,
                cwd=td, env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
            )
            passed = proc.returncode == 0
            tail = (proc.stdout[-300:] + proc.stderr[-150:])
        except subprocess.TimeoutExpired:
            passed, tail = False, "TIMEOUT"
    return {"passed": passed, "tail": tail.strip()}


def check_candidate(code: str, task: dict) -> dict:
    if task["kind"] == "creation":
        return run_pytest_suite(code, task["suite"])
    # Canonical HumanEval harness: tests define check(candidate); invoke it
    # directly so a missing/renamed entry point FAILS instead of being swallowed.
    program = task["prompt"] + "\n" + code + "\n" + task["test"] + \
        f"\ncheck({task['entry_point']})\n"
    return run_contracts(program, None, task["entry_point"])


# --------------------------------------------------------------- PPT tournament
def ppt_compare(a_code: str, b_code: str, spec: str, rng_salt: int, cheap_model: str = "gpt-4o-mini") -> bool:
    """Returns True when candidate A wins. Cheap-tier judge, temp 0."""
    prompt = (
        "You are a strict code reviewer. Compare two candidate implementations "
        "against the task requirements.\n\n"
        f"TASK:\n{spec[:1200]}\n\nCANDIDATE A:\n{a_code[:2200]}\n\n"
        f"CANDIDATE B:\n{b_code[:2200]}\n\n"
        'Reply ONLY with JSON: {"winner": "A"} or {"winner": "B"}. '
        "Judge correctness against requirements first, then robustness."
    )
    text, _ = chat(cheap_model, [{"role": "user", "content": prompt}], 0.0)
    b_wins = '"winner": "B"' in text.replace("'", '"') or '"winner":"B"' in text
    return not b_wins  # mirrors Rust default-bias semantics


def tournament_comparison_count(n: int, k_req: int) -> int:
    k = max(1, min(k_req, max(n // 2, 1)))
    return n + (n - k) * k + k * (k - 1) // 2


def ppt_run(codes: list[str], spec: str, cheap_model: str = "gpt-4o-mini") -> tuple[int, list[tuple[int, float]]]:
    """Ring pass + pivot tournament. Returns (winner_idx, win_rates)."""
    n = len(codes)
    k = max(1, min(K_PIVOTS_REQUESTED, max(n // 2, 1)))
    wins = {i: 0 for i in range(n)}
    matches = {i: 0 for i in range(n)}

    def match(i: int, j: int) -> None:
        a_won = ppt_compare(codes[i], codes[j], spec, i * 31 + j, cheap_model=cheap_model)
        winner, loser = (i, j) if a_won else (j, i)
        wins[winner] += 1
        matches[i] += 1
        matches[j] += 1

    pairs = [(i, (i + 1) % n) for i in range(n)]  # ring
    pivots = list(range(k))  # deterministic pivot choice for pilot reproducibility
    pivot_set = set(pivots)
    for i in range(n):
        if i in pivot_set:
            continue
        for p in pivots:
            pairs.append((i, p))
    for x in range(len(pivots)):
        for y in range(x + 1, len(pivots)):
            pairs.append((pivots[x], pivots[y]))
    for i, j in pairs:
        match(i, j)

    rates = [(i, wins[i] / max(matches[i], 1)) for i in range(n)]
    winner = max(rates, key=lambda t: t[1])[0]
    return winner, rates


def is_tie(rates: list[tuple[int, float]], eps: float = TIE_EPSILON) -> bool:
    top = sorted((r for _, r in rates), reverse=True)
    return len(top) >= 2 and (top[0] - top[1]) <= eps


# ------------------------------------------------------------------- task sets
def load_humaneval(limit: int) -> list[dict]:
    cache = HERE / "results" / "humaneval.jsonl"
    cache.parent.mkdir(exist_ok=True)
    if not cache.exists():
        log("Downloading HumanEval dataset...")
        raw = urllib.request.urlopen(HUMANEVAL_URL, timeout=60).read()
        text = gzip.decompress(raw).decode()
        cache.write_text(text, encoding="utf-8")
    tasks = []
    for line in cache.read_text(encoding="utf-8").splitlines():
        row = json.loads(line)
        tasks.append({
            "kind": "humaneval", "id": row["task_id"], "spec": row["prompt"],
            "prompt": row["prompt"], "test": row["test"],
            "entry_point": row["entry_point"],
        })
        if len(tasks) >= limit:
            break
    return tasks


def load_creation_tasks() -> list[dict]:
    sys.path.insert(0, str(HERE))
    from tasks_creation import CREATION_TASKS
    return [{
        "kind": "creation", "id": t["id"], "spec": t["description"],
        "suite": t["suite"],
    } for t in CREATION_TASKS]


# ------------------------------------------------------------------ generation
def generate(spec: str, model: str, temperature: float, provider: str | None = None) -> tuple[str, float]:
    prompt = (
        "Implement the following task. Reply with ONLY one Python code block.\n\n"
        f"{spec}"
    )
    text, _ = chat(model, [{"role": "user", "content": prompt}], temperature, provider)
    return extract_code(text), 0.0


def refine_with_flagship(spec: str, code: str, failure_tail: str, flagship_model: str = "deepseek-v4-flash") -> str:
    prompt = (
        "The following implementation FAILS its formal I/O contract suite.\n"
        f"TASK:\n{spec[:1200]}\n\nCURRENT IMPLEMENTATION:\n{code[:2400]}\n\n"
        f"CONTRACT FAILURE OUTPUT (counterexample):\n{failure_tail}\n\n"
        "Fix the implementation. Reply with ONLY the corrected full Python code block."
    )
    text, _ = chat(
        flagship_model,
        [{"role": "user", "content": prompt}], 0.2, "deepseek",
    )
    return extract_code(text)


# ----------------------------------------------------------------------- pilot
def pilot_task(task: dict, out_rows: list, cheap_model: str = "gpt-4o-mini", flagship_model: str = "deepseek-v4-flash") -> None:
    tid = task["id"]
    log(f"\n=== {tid} ({task['kind']}) ===")

    # ---- config B: cheap-only single shot
    code_b, _ = generate(task["spec"], cheap_model, 0.2)
    res_b = check_candidate(code_b, task)

    # ---- config A: flagship-only single shot
    code_a, _ = generate(task["spec"], flagship_model, 0.2)
    res_a = check_candidate(code_a, task)

    # ---- MAIN tiered run: N cheap drafts
    drafts, draft_results = [], []
    for i, temp in enumerate(DRAFT_TEMPERATURES):
        code, _ = generate(task["spec"], cheap_model, temp)
        verdict = check_candidate(code, task)
        drafts.append(code)
        draft_results.append(verdict)
        log(f"  draft[{i}] temp={temp}: {'PASS' if verdict['passed'] else 'FAIL'}")

    n_passing = sum(1 for d in draft_results if d["passed"])
    random_expected = n_passing / N_DRAFTS                      # config C
    first_passing = next((i for i, d in enumerate(draft_results) if d["passed"]), None)
    verify_all_pass = first_passing is not None                 # config V

    # ---- PPT tournament over drafts (cheap judge)
    winner_idx, rates = ppt_run(drafts, task["spec"], cheap_model=cheap_model)
    tie = is_tie(rates)
    ppt_passes = draft_results[winner_idx]["passed"]

    # ---- escalation policy: elected winner failed -> flagship refines once
    escalated, refined_pass = False, None
    final_code = drafts[winner_idx]
    if not ppt_passes:
        escalated = True
        fixed = refine_with_flagship(task["spec"], final_code, draft_results[winner_idx]["tail"], flagship_model=flagship_model)
        final_code = fixed
        refined_res = check_candidate(fixed, task)
        refined_pass = refined_res["passed"]

    final_pass = ppt_passes or bool(refined_pass)
    log(f"  PPT->draft[{winner_idx}] {'PASS' if ppt_passes else 'FAIL'} | tie={tie} "
        f"| escalate={escalated} refined={'PASS' if refined_pass else ('FAIL' if refined_pass is False else '-')}")

    out_rows.append({
        "task_id": tid, "kind": task["kind"],
        "B_cheap_pass": res_b["passed"], "A_flagship_pass": res_a["passed"],
        "C_random_expected_pass": random_expected,
        "V_verify_all_pass": verify_all_pass,
        "T_ppt_pick_pass": ppt_passes, "ppt_winner_idx": winner_idx,
        "tournament_tie": tie, "rates": rates,
        "n_drafts_passing": n_passing,
        "escalated": escalated,
        "refined_pass": refined_pass,
        "D_full_pass": final_pass,
        "budget_snapshot": {k: round(v, 5) for k, v in BUDGET.spend.items()},
    })


def wilson_ci(p: float, n: int, z: float = 1.96) -> tuple[float, float]:
    if n == 0:
        return (0.0, 0.0)
    denom = 1 + z * z / n
    centre = (p + z * z / (2 * n)) / denom
    half = z * ((p * (1 - p) / n + z * z / (4 * n * n)) ** 0.5) / denom
    return (max(0.0, centre - half), min(1.0, centre + half))


def summarize(rows: list[dict], cheap_model: str = "gpt-4o-mini", flagship_model: str = "deepseek-v4-flash") -> None:
    n = len(rows)
    if n == 0:
        return

    def pct(name: str) -> float:
        vals = {
            "B": lambda r: r["B_cheap_pass"], "A": lambda r: r["A_flagship_pass"],
            "V": lambda r: r["V_verify_all_pass"], "T": lambda r: r["T_ppt_pick_pass"],
            "D": lambda r: r["D_full_pass"],
        }[name]
        return sum(1 for r in rows if vals(r)) / n

    def c_exp() -> float:  # expected pass under random pick (fractional)
        return statistics.fmean(r["C_random_expected_pass"] for r in rows)

    log("\n" + "=" * 64)
    log(f"RESULTS (n={n} tasks)")
    log("=" * 64)
    labels = [("B", f"cheap-only ({cheap_model})"), ("A", f"flagship-only ({flagship_model})"),
              ("V", "verify-all-first-pass"), ("C*", "best-of-3 RANDOM (expected)"),
              ("T", "PPT pick, no escalation"), ("D", f"FULL tiered + {flagship_model}")]
    for key, name in labels:
        p = c_exp() if key == "C*" else pct(key)
        lo, hi = wilson_ci(p, n)
        bar = "#" * int(p * 40)
        log(f"  {name:<32} {p*100:5.1f}%  [{lo*100:.1f}-{hi*100:.1f}]  {bar}")

    # Q1: paired T vs C (per-task fractional random baseline)
    deltas_q1 = [r["T_ppt_pick_pass"] - r["C_random_expected_pass"] for r in rows]
    m_q1 = statistics.fmean(deltas_q1)
    # bootstrap CI on mean delta
    rng = random.Random(7)
    boots = []
    for _ in range(2000):
        sample = [deltas_q1[rng.randrange(n)] for _ in range(n)]
        boots.append(statistics.fmean(sample))
    boots.sort()
    log(f"\nQ1  PPT vs RANDOM  : {m_q1:+.1%} pts  "
        f"[{boots[49]:+.1%}, {boots[1949]:+.1%}] (95%)")

    # Q2: paired D vs T
    deltas_q2 = [r["D_full_pass"] - r["T_ppt_pick_pass"] for r in rows]
    m_q2 = statistics.fmean(deltas_q2)
    esc = [r for r in rows if r["escalated"]]
    precision = (sum(1 for r in esc if not r["T_ppt_pick_pass"]) / len(esc)) if esc else float("nan")
    fixes = sum(1 for r in esc if r["refined_pass"]) 
    log(f"Q2  +{flagship_model}      : {m_q2:+.1%} pts  | escalations={len(esc)} "
        f"(precision {precision:.0%}), fixes={fixes}/{len(esc)}")

    log("\nSpend:")
    for model, usd in BUDGET.spend.items():
        log(f"  {model:<22} ${usd:.4f}")
    log(f"  {'TOTAL':<22} ${BUDGET.total():.4f}")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cheap-model", default=os.environ.get("OPENAI_MODEL", "gpt-4o-mini"),
                    help="cheap model for drafting and comparisons (e.g. gpt-3.5-turbo, gpt-4o-mini)")
    ap.add_argument("--flagship-model", default=os.environ.get("DEEPSEEK_MODEL", "deepseek-chat"),
                    help="flagship model for escalation refinement (e.g. deepseek-v4-flash, deepseek-chat)")
    ap.add_argument("--limit", type=int, default=56)
    ap.add_argument("--skip-creation", action="store_true")
    ap.add_argument("--smoke", action="store_true", help="3 humaneval tasks only")
    ap.add_argument("--resume", metavar="RESULTS_JSONL", default=None,
                    help="resume: skip task_ids already present in the given "
                         "results file and append new rows to it")
    args = ap.parse_args()
    if args.smoke:
        args.limit = 3
        args.skip_creation = True

    RESULTS_DIR.mkdir(exist_ok=True)
    tasks = load_humaneval(args.limit)
    if not args.skip_creation:
        tasks += load_creation_tasks()

    out_path = RESULTS_DIR / f"results_{int(time.time())}.jsonl"
    rows: list[dict] = []
    done_ids: set[str] = set()
    if args.resume:
        src = Path(args.resume)
        if not src.exists():
            sys.exit(f"--resume: file not found: {src}")
        rows = [json.loads(l) for l in src.read_text(encoding="utf-8").splitlines() if l.strip()]
        done_ids = {r["task_id"] for r in rows}
        out_path = src
        todo = [t for t in tasks if t["id"] not in done_ids]
        if len(todo) < len(tasks):
            log(f"Resume: {len(done_ids)} tasks already done, {len(todo)} remaining "
                f"-> writing back to {src.name}")
        tasks = todo
    log(f"Pilot: {len(tasks)} tasks | cheap: {args.cheap_model} | flagship: {args.flagship_model} | "
        f"caps: ${BUDGET_CAP_USD}/provider | N={N_DRAFTS} k={K_PIVOTS_REQUESTED}")
    try:
        for t in tasks:
            pilot_task(t, rows, cheap_model=args.cheap_model, flagship_model=args.flagship_model)
            with open(out_path, "w", encoding="utf-8") as f:
                for r in rows:
                    f.write(json.dumps(r) + "\n")
    except (KeyboardInterrupt, SystemExit):
        log("interrupted; partial results retained")
    finally:
        summarize(rows, cheap_model=args.cheap_model, flagship_model=args.flagship_model)
        log(f"\nPer-task log saved to: {out_path}")


if __name__ == "__main__":
    main()
