# 📜 Changelog

All notable changes to DSpark will be documented in this file.

---

## [0.3.0] - 2026-08-28

### Added
- **AgentDeltaMemory (KDA-derived agent memory)**: delta-rule writes (correct-only-on-error), per-channel decay (`invariant`/`decision`/`transient`), key-bound rank-1 updates (DPLR binding), and a delta→0 convergence theorem used as a memory-stable early stop in the CEGAR loop. Rust + Python mirrors.
- **Verification scaling (LLM-as-a-Verifier, arXiv:2607.05391)**:
  - PPT **soft updates** (Bradley-Terry of 1-20 scores, binary fallback, EQUAL→0.5);
  - **continuous reward** (Eq. 3.1: expectation over scoring-token logits, A–T/1–20 vocabulary) + two-stage workaround for logit-restricted APIs (B.6);
  - **criteria decomposition** (Specification/Output/Errors) in the Curator + `criteria_scores`;
  - **repeated evaluation K** (`curator_repetitions`) with conservative verdict aggregation;
  - **VOC progress signal** (Spearman iteration×score) + stagnation early stop in the CEGAR loop.
- **Confidence scheduling (DSpark, arXiv:2607.05147)**:
  - **STS calibration** (`StsCalibrator`): sequential temperature scaling of cumulative prefix survival (logit transform, ECE grid search);
  - **greedy early-stop scheduler** (`CostScheduler::with_early_stop`) with `expected_accepted` and a documented non-anticipation invariant;
  - **sequential dependency pass** (`--sequential`, default on): prefix-conditioned re-draft of parallel trajectories (sequential-head analog).
- **Offline A/B harnesses** (deterministic, CI-asserted): `tests/verification_scaling_test.rs` (E1–E4) and `bench/compare_cegar_improvements.py` (B1–B3); `tests/test_bench_claims.py` guards them.
- **Real-bench upgrades**: dual-policy PPT (discrete 1-5 vs soft 1-20 on the same calls), N=5 drafts, `--judge-model` tier separation, `bench/show_results.py`, `bench/compare_vt.py`.

### Fixed
- **Memory key collision bug**: counterexample/task/contract keys shared too many tokens (cosine 0.75/0.67 > 0.45) causing distinct counterexamples to bind to the same memory entry and wrongly trigger the memory-stable early stop. Keys now use digest chunks dominating the embedding.
- `asyncio.run` nesting in the offline bench; global config leakage between tests.

### Measured (offline, CI-asserted)
- PPT soft: +2 pts selection accuracy, ties→0 (E1); continuous reward: 84%→100%, ties→0 (E2); STS: ECE −85% (E3); scheduler: −22% calls (E4); memory+VOC: 189→79 iterations (−58%) with outcome parity; K=3: score MAE −66%.

### Measured (real APIs, 48 tasks, ≈$0.05)
- Tiered pipeline (drafts + tournament + escalation): **100% pass** across both cheap tiers.
- **Judge-tier finding**: judging drafts with the SAME model tier gives PPT 70% vs first-pass 90%; a strictly stronger judge gives PPT 100% (+22.5 pts over random). CLI and bench now warn when judge tier == draft tier.

---

## [0.2.1] - 2026-08-23

### Added
- **Dynamic Reasoning Token Budgeting**: Automated headroom calculation (`max_tokens=4096`) for reasoning models (`deepseek-v4-flash`, `deepseek-chat`) to prevent truncation caused by internal `reasoning_content`.
- **Flexible Multi-Model Benchmarking**: Added `--cheap-model` and `--flagship-model` CLI parameters with auto-detected provider endpoints to `bench/run_real_bench.py`.
- **Live Empirical Pilot Validation**: Full 12-task benchmark demonstrating +33.3% quality improvement on weak models (`gpt-3.5-turbo`) and 100% pass rate on flagship models (`deepseek-chat`).
- **Comprehensive IDE MCP Guides**: Step-by-step FastMCP configuration for Cursor, Windsurf, Claude Desktop, Claude Code, and Antigravity.

---

## [0.2.0] - 2026-08-22

### Added
- **Speculative Orchestration Engine**: Parallel multi-trajectory drafting with bounded `tokio::sync::Semaphore`.
- **Probabilistic Pivot Tournament (PPT)**: $O(Nk)$ trajectory tournament algorithm.
- **Local Confidence Head**: Shannon entropy and cyclomatic complexity estimation.
- **Cost-Aware Scheduler**: Budget-conscious verification pruning saving 60–98% of API calls.
- **Pluggable AST Resolver**: `DependencyResolver` trait with `RegexResolver` and `TreeSitterResolver` feature flag backends.
- **Criterion Benchmark Suite**: Scaling benchmarks for PPT tournament and pruning efficiency.
- **Python 3.10 Compatibility**: Seamless polyfills for typing and lazy litellm imports.

---

## [0.1.0] - 2026-08-20

### Added
- Initial dual-engine CEGAR pipeline with Gemini Creator and DeepSeek Curator.
- FastMCP server implementation for Cursor and Claude Code.
- Isolated subprocess sandbox runner for adversarial test execution.
