<div align="center">

# 🚀 DSpark

**Agent-Level Speculative Orchestration & Formal Dual-Engine Code Generation**

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/CostaJr007/dspark/actions)
[![Tests](https://img.shields.io/badge/tests-37%20Rust%20%2B%2016%20Python-brightgreen)](https://github.com/CostaJr007/dspark/actions)
[![Live Pilot](https://img.shields.io/badge/live_pilot-91.1%25%20pass%401%20%240.108-blue)](#-live-pilot--real-models-real-spend)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://rust-lang.org)
[![Python](https://img.shields.io/badge/python-3.10%2B-blue)](https://python.org)

[📚 Documentation](docs/) • [🚀 Quick Start](#-quick-start) • [📊 Benchmarks](#-benchmarks) • [🤝 Contributing](docs/CONTRIBUTING.md) • [🇧🇷 Português](README.pt-BR.md)

</div>

---

## 🎯 What is DSpark?

**DSpark** is an enterprise-grade AI coding platform that implements **Agent-Level Speculative Orchestration**, combining:

- 🧠 **Dual-Engine Architecture (CEGAR)**: Epistemically isolates code drafting (Creator) from adversarial verification (Curator) to overcome the *Self-Correction Fallacy* (Huang et al., 2023).
- ⚡ **Semi-Autoregressive Speculative Decoding**: Generates $N$ parallel code trajectories using bounded concurrency, validated by AST dependency resolution.
- 🔍 **Probabilistic Pivot Tournament (PPT)**: Evaluates trajectories in $O(Nk)$ comparisons instead of naive all-pairs $O(N^2)$ based on fine-grained reward estimation.
- 📊 **Confidence-Scheduled Pruning**: Analyzes local CPU cyclomatic entropy to prune 40–70% of redundant API calls without compromising safety.
- 🔌 **FastMCP Server**: Integrates directly as a Model Context Protocol server for Cursor, Windsurf, Claude Code, Roo Code, and Antigravity.

> **Theoretical Foundations**: [DSpark (DeepSeek & Peking University, 2026)](https://arxiv.org/abs/2607.05147) and [LLM-as-a-Verifier (Kwok et al., 2026)](https://arxiv.org/abs/2607.05391).

### 🏆 Key Benchmarks & Results

| Metric | DSpark Speculative | Naive Round-Robin | Improvement |
|---|---|---|---|
| **API Comparisons ($N=100$, $k=3$)** | 394 | 4,950 | **92.0% reduction** ✅ reproducible |
| **API Comparisons ($N=50$, $k=3$)** | 194 | 1,225 | **84.2% reduction** ✅ reproducible |
| **API Comparisons ($N=20$, $k=3$)** | 74 | 190 | **61.1% reduction** ✅ reproducible |
| **Local Verification Pruning** | 60–98% pruned (budget-capped) | 0% | hard spend ceiling ✅ reproducible |
| **Code Pass Rate, tiered vs flagship-only** | simulated: 96.0% vs 90.5% @ ~58% cost † | — | hypothesis, validate on your workload |
| **Test Suite Coverage** | 37 Rust + 16 Python tests green in CI | - | ✅ |

✅ = asserted in CI by `tests/tournament_scaling_test.rs` and
`tests/pruning_reproducibility_test.rs` (methodology: [docs/BENCHMARKS.md](docs/BENCHMARKS.md)).
† = output of the declared-assumption simulation in `examples/cost_quality_harness.rs`;
not a measured model-accuracy result — do not quote as one.

> PPT pays off for **N ≥ 10**; below that the ring-pass overhead exceeds all-pairs cost.

### 🔬 Live Pilot — Real Models, Real Spend

A single end-to-end run of the full tiered pipeline against **live APIs**
(56 tasks: 50 HumanEval + 6 open-ended code creation, sandbox-graded,
2026-08-22, total spend **US$ 0.108**):

| Configuration | pass@1 | Notes |
|---|---|---|
| Cheap-only (`gpt-4o-mini`) | 89.3% | baseline B |
| Flagship-only (`deepseek-v4-flash`) | 83.9% | baseline A |
| Best-of-3 random pick | 89.3% | counterfactual from same drafts |
| Verify-all, first passing (free) | 89.3% | local sandbox grading only |
| **PPT pick (no escalation)** | 89.3% | tournament selection |
| **Tiered complete (PPT + escalation)** | **91.1%** | flagship refines hard cases |

Honest reading of this pilot:

- **Q1 — does the tournament add quality?** Here: **+0.0 pts**. In 0 of 56
  tasks did the three drafts disagree (all passed or all failed together),
  so there was nothing for the tournament to separate. This measures the
  *benchmark regime*, not a defect in PPT.
- **Q2 — does the flagship escalation add value?** **+1.8 pts**: the
  escalation policy targeted exactly the 6 failing tasks (100% precision)
  and repaired 1 of them. Directionally positive; n=56 is too small for
  significance.
- The structural wins above (comparison counts, pruning, cost ceilings) are
  where DSpark's savings are already proven.

Reproduce it: `python bench/run_real_bench.py` (requires `OPENAI_API_KEY`
and `DEEPSEEK_API_KEY`; per-task JSONL logs land in `bench/results/`).

---

## ✨ Features

### 🔥 Core Capabilities
- **Speculative Drafting**: Generates $N$ diverse code candidates in parallel using `tokio::spawn` bounded by semaphores.
- **Topological AST Invariant Ordering**: Resolves function call graphs using `petgraph` DAGs to eliminate circular dependency bugs before verification.
- **Probabilistic Pivot Tournament**: Selects $k$ pivots from an adjacent Ring Pass ($i \to i+1 \pmod N$) and executes non-pivot tournaments in $O(Nk)$ time.
- **Local Confidence Head**: Computes cyclomatic entropy, state mutation, and safety risks locally on CPU in $<1\text{ms}$.
- **Pluggable AST Backends**: Switch between high-speed `regex-ast` (default) and full `tree-sitter-ast` via Cargo feature flags.
- **Resilient JSON & Output Repair**: 4-layer fallback parser resilient to markdown fences and malformed LLM outputs.
- **Epistemic Isolation**: Curator sees ONLY source code and formal I/O contracts (zero creator thought leaks).

---

## 🚀 Quick Start

### Prerequisites
- **Rust**: 1.75+ ([install](https://rustup.rs/))
- **Python**: 3.10+ (for Python SDK & pytest sandbox runner)
- **API Keys**: DeepSeek API Key, OpenAI API Key, or Google Gemini API Key

### Installation

```bash
# Clone the repository
git clone https://github.com/CostaJr007/dspark.git
cd dspark

# Install the Rust CLI (fast regex AST backend)
cargo install --path crates/dspark-core --force

# OR install with Tree-Sitter AST support
cargo install --path crates/dspark-core --features tree-sitter-ast --force

# Install the Python SDK
pip install -e .
```

### Configuration

```bash
# Set environment variables
export DEEPSEEK_API_KEY="your-deepseek-api-key"
export OPENAI_API_KEY="your-openai-api-key"
export GEMINI_API_KEY="your-gemini-api-key"

# Configure model pair
dspark pair --creator gpt-4o-mini --curator deepseek-chat
```

### Running Speculative Orchestration

```bash
# Execute multi-trajectory speculative orchestration (N=4 trajectories, k=2 pivots)
dspark run "Implement a thread-safe LRU Cache with TTL expiration in Python" \
           --speculative \
           --trajectories 4 \
           --pivots 2 \
           --ranking-model deepseek-chat \
           --out lru_cache.py
```

Tiered routing: the **cheap tier** drafts trajectories AND runs tournament
comparisons (`--ranking-model`, defaults to the creator model); the flagship
`--curator` is invoked only when the escalation policy detects a residual hard
case (tournament tie, unverified high-risk blocks, low winner confidence,
invalid AST).

### Running CEGAR Audit & Refinement (MCP)

```bash
# Start the FastMCP Server
dspark-mcp
```

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                       DSpark Speculative Engine                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   [User Spec] ────► [Speculative Drafter] ────► N Candidate Drafts     │
│        │                   │                                            │
│        │                   ▼                                            │
│        │        [AST Dependency Resolver] ────► Topo-Sorted DAG Code    │
│        │                   │                                            │
│        │                   ▼                                            │
│        │           [Confidence Head] ─────────► Local Risk & Entropy    │
│        │                   │                    (Prunes 40-70% trivial) │
│        │                   ▼                                            │
│        │            [Cost Scheduler] ─────────► Verification Budget     │
│        │                   │                                            │
│        │                   ▼                                            │
│        │       [Pivot Tournament (PPT)] ──────► O(Nk) Comparisons       │
│        │                   │                                            │
│        │                   ▼                                            │
│        └──────────► [CEGAR Refiner] ──────────► Final Verified Code     │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 📖 Documentation Index

| Document | Description |
|---|---|
| [🏛️ Architecture](docs/ARCHITECTURE.md) | In-depth engineering specifications of the 5-stage pipeline |
| [🚀 Getting Started](docs/GETTING_STARTED.md) | Step-by-step setup, configuration, and first execution |
| [🔌 API Reference](docs/API_REFERENCE.md) | Full Rust crate and Python SDK API reference |
| [⌨️ CLI Reference](docs/CLI_REFERENCE.md) | Complete CLI arguments, options, and commands |
| [📊 Benchmarks](docs/BENCHMARKS.md) | Criterion scaling benchmarks and methodology |
| [🎓 Theory](docs/THEORY.md) | Academic background (DSpark, CEGAR, LLM-as-a-Verifier) |
| [🤝 Contributing](docs/CONTRIBUTING.md) | Contribution guidelines, code standards, and PR workflows |
| [📜 Changelog](docs/CHANGELOG.md) | Version history and milestone releases |

---

## 📊 Benchmarks

Run the complete Criterion test suite:

```bash
./scripts/bench_all.sh
```

### Tournament Scaling ($k=3$ Pivots)
```
N=10:  O(Nk)=34 comparisons  vs all-pairs=45   (24.4% savings; PPT overhead below N>=10)
N=20:  O(Nk)=74 comparisons  vs all-pairs=190  (61.1% savings)
N=50:  O(Nk)=194 comparisons vs all-pairs=1225 (84.2% savings)
N=100: O(Nk)=394 comparisons vs all-pairs=4950 (92.0% savings)
```

### Live Pilot (56 tasks, US$ 0.108)
```
cheap-only gpt-4o-mini        : 89.3% pass@1
flagship-only deepseek-v4-flash: 83.9%
tiered (PPT + escalation)     : 91.1%   (+1.8 pts from escalation)
tournament vs random          : +0.0    (drafts perfectly correlated in this regime)
escalation precision          : 6/6 targeted true failures, 1 repaired
```

---

## 🧪 Testing

```bash
# Run all Rust tests (37 tests)
cargo test -p dspark-core

# Run all Python tests (16 tests)
python -m unittest discover tests
pytest -v
```

---

## 📜 License

Distributed under the **MIT License**. See `LICENSE` for details.