# 🚀 DSpark Master Plan: Agent-Level Speculative Orchestration

## Implementing Speculative Decoding (DeepSeek) + LLM-as-a-Verifier in dspark-core

---

## 📋 1. OVERVIEW & OBJECTIVES

### 1.1 Context
This document specifies the evolution of `dspark-core` from a dual-engine verification tool into an **Agent-Level Speculative Orchestration System**, synthesizing the methodologies of:
- **DSpark** (DeepSeek & Peking University, 2026): *Confidence-Scheduled Speculative Decoding*
- **LLM-as-a-Verifier** (Kwok et al., 2026): *Probabilistic Pivot Tournament* and *Fine-Grained Reward Estimation*

### 1.2 Core Objectives
Transform `dspark run` into a pipeline that:
1. ✅ Generates multiple code trajectories concurrently (*Semi-Autoregressive Drafting*)
2. ✅ Validates syntax and topologically orders dependencies via AST before verification (*Sequential Dependency Injection*)
3. ✅ Estimates risk and cyclomatic entropy locally on CPU to avoid redundant API calls (*Confidence-Scheduled Verification*)
4. ✅ Ranks candidates via an $O(Nk)$ tournament algorithm (*Probabilistic Pivot Tournament*)
5. ✅ Extracts fine-grained logprob rewards for dense token feedback (*Fine-Grained Reward via Logprobs*)
6. ✅ Optimizes prompt ordering to maximize KV-cache hit rates (*Prefix-Cache Optimization*)

### 1.3 Expected Benefits
- **60–98% reduction** in API costs via confidence-scheduled verification pruning
- **Up to 50% lower wall-clock latency** via speculative async concurrency
- **Proven pass rate improvements** (boosting weak models from 41.7% to 75.0%, and flagship models to 100%)

---

## 🏗️ 2. PROPOSED ARCHITECTURE

### 2.1 Pipeline Data Flow

```
[Task Prompt + I/O Contracts]
            |
            v
[STAGE 1: Speculative Drafter] --(N=3-5 trajectories)--> [Pool of Drafts]
            |                                                    |
            | [Tree-sitter Sequential Module]                    |
            v                                                    v
[STAGE 2: Dependency Resolver] --(valid drafts only)--> [Valid Draft Pool]
            |                                                    |
            | [Local Confidence Head]                            |
            v                                                    v
[STAGE 3: Cost Scheduler] --(high-risk blocks only)--> [Verification Batch]
            |                                                    |
            v                                                    v
[STAGE 4: Pivot Tournament] --(O(Nk) comparisons)--> [Best Trajectory]
            |
            v
[STAGE 5: CEGAR Sandbox Refiner] --(concrete tracebacks)--> [Verified Code]
```

### 2.2 Directory Structure

```
crates/dspark-core/src/
├── lib.rs                          # Entry point
├── cli/                            # CLI Commands
│   ├── mod.rs
│   ├── run.rs                      # Speculative execution mode
│   ├── audit.rs
│   ├── refine.rs
│   └── pair.rs
├── engine/                         # Core Orchestrator Engine
│   ├── mod.rs
│   ├── speculative_drafter.rs      # Concurrent drafting + semaphore control
│   ├── confidence_head.rs          # Local CPU entropy and risk heuristics
│   ├── cost_scheduler.rs           # Cost/Hardware-Aware Budget Scheduler
│   ├── pivot_tournament.rs         # PPT Tournament O(Nk) algorithm
│   └── logprob_extractor.rs        # Logprob extraction and reward decomposition
├── verifier/                       # Verification clients
│   ├── mod.rs
│   ├── contracts.rs
│   └── deepseek_client.rs          # DeepSeek client with logprob support
├── creator/                        # Drafting clients
│   ├── mod.rs
│   └── gemini_client.rs
└── utils/
    ├── prompt_optimizer.rs         # Prefix-cache friendly prompt formatting
    └── ast_resolver.rs             # Tree-sitter & Regex dependency DAG resolver
```

---

## 📦 3. DEPENDENCIES AND TOOLCHAIN

### 3.1 `crates/dspark-core/Cargo.toml`

```toml
[dependencies]
tokio = { version = "1.0", features = ["full", "sync"] }
reqwest = { version = "0.11", features = ["json", "blocking"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tree-sitter = "0.22"
tree-sitter-rust = "0.21"
tree-sitter-python = "0.21"
petgraph = "0.6"
ordered-float = "4.2"
```

---

## 📅 4. EXECUTION PHASES

- **Phase 1: Foundation & Drafting**: `Cargo.toml`, `utils/ast_resolver.rs`, `engine/speculative_drafter.rs`.
- **Phase 2: Confidence Head & Cost Scheduler**: `engine/confidence_head.rs`, `engine/cost_scheduler.rs`.
- **Phase 3: Probabilistic Pivot Tournament**: `engine/pivot_tournament.rs` ($O(Nk)$ vs $O(N^2)$).
- **Phase 4: Logprobs & Prefix Cache**: `engine/logprob_extractor.rs`, `utils/prompt_optimizer.rs`.
- **Phase 5: CLI & E2E Verification**: `cli/run.rs` (`--speculative`), test suite, and benchmarks.

---
