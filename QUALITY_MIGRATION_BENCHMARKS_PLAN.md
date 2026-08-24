# 🏗️ DSpark: Quality, Migration, and Benchmarking Plan
## Complement to MASTER_PLAN_SPECULATIVE_ORCHESTRATION.md

This document specifies the execution plan for enterprise-grade verification, test coverage, and benchmark methodology for DSpark.

---

# 📋 PART 1: UNIT TEST SPECIFICATION & COVERAGE TARGETS

## 1.1 Target Coverage by Module

| Module | Lines | Target Coverage | Priority | Complexity |
|---|---|---|---|---|
| `pivot_tournament.rs` | 226 | 95% | 🔴 P0 | High |
| `speculative_drafter.rs` | 91 | 90% | 🔴 P0 | High |
| `logprob_extractor.rs` | 136 | 90% | 🟡 P1 | Medium |
| `cost_scheduler.rs` | 78 | 85% | 🟡 P1 | Low |
| `confidence_head.rs` | 118 | 85% | 🟡 P1 | Low |

## 1.2 Test File Hierarchy

```
crates/dspark-core/
├── src/
│   ├── engine/
│   │   ├── pivot_tournament.rs
│   │   ├── speculative_drafter.rs
│   │   ├── logprob_extractor.rs
│   │   ├── cost_scheduler.rs
│   │   └── confidence_head.rs
│   └── ...
└── tests/
    ├── engine/
    │   ├── mod.rs
    │   ├── pivot_tournament_test.rs
    │   ├── speculative_drafter_test.rs
    │   ├── logprob_extractor_test.rs
    │   ├── cost_scheduler_test.rs
    │   └── confidence_head_test.rs
    ├── utils/
    │   ├── mod.rs
    │   └── ast_resolver_test.rs
    └── mocks/
        ├── mod.rs
        └── mock_client.rs
```

---

# 🌲 PART 2: REGEX → TREE-SITTER MIGRATION

## 2.1 Technical Rationale
- Precise syntax tree parsing, transitive dependency detection, and elimination of false positives inside string literals and docstrings.
- Feature flags: `default = ["regex-ast"]` for lightning-fast compilation, and `tree-sitter-ast` for rigorous semantic verification.

---

# 📊 PART 3: CRITERION BENCHMARK METHODOLOGY

## 3.1 Benchmark Objectives
1. Empirically prove $O(Nk)$ vs $O(N^2)$ scaling in the Pivot Tournament.
2. Measure local pruning rates via the CPU-based Confidence Head (60–98% savings under budget constraints).
3. Compare Regex vs Tree-Sitter parsing latency and throughput.
4. Measure prompt prefix-caching hit rates (up to 80% KV cache savings).
