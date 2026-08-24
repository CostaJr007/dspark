# 📜 Changelog

All notable changes to DSpark will be documented in this file.

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
