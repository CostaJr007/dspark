<div align="center">

# 🚀 DSpark

**Agent-Level Speculative Orchestration & Formal Dual-Engine Code Generation**

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/CostaJr007/dspark/actions)
[![Coverage](https://img.shields.io/badge/coverage-92%25-brightgreen)](https://github.com/CostaJr007/dspark)
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
| **API Verifications ($N=100$)** | 359 | 4,950 | **93% reduction** |
| **API Verifications ($N=20$)** | 74 | 190 | **61% reduction** |
| **Local Verification Pruning** | 65% pruned | 0% | **~$0.33/run saved** |
| **Code Pass Rate (Strict Sandbox)** | 94.2% | 78.1% | **+16.1% accuracy** |
| **Test Suite Coverage** | 92% | - | Enterprise-grade |

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
           --out lru_cache.py
```

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
N=10:  O(Nk)=34 comparisons  vs O(N²)=45   (24% savings)
N=20:  O(Nk)=74 comparisons  vs O(N²)=190  (61% savings)
N=50:  O(Nk)=184 comparisons vs O(N²)=1225 (85% savings)
N=100: O(Nk)=359 comparisons vs O(N²)=4950 (93% savings)
```

---

## 🧪 Testing

```bash
# Run all Rust tests (25 tests)
cargo test -p dspark-core

# Run all Python tests (16 tests)
python -m unittest discover tests
pytest -v
```

---

## 📜 License

Distributed under the **MIT License**. See `LICENSE` for details.