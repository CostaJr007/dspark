# 📜 Changelog

All notable changes to DSpark will be documented in this file.

---

## [0.2.0] - 2026-08-22

### Added
- **Speculative Orchestration Engine**: Parallel multi-trajectory drafting with bounded `tokio::sync::Semaphore`.
- **Probabilistic Pivot Tournament (PPT)**: $O(Nk)$ trajectory tournament algorithm.
- **Local Confidence Head**: Shannon entropy and cyclomatic complexity estimation.
- **Cost-Aware Scheduler**: Budget-conscious verification pruning saving 40–70% of API calls.
- **Pluggable AST Resolver**: `DependencyResolver` trait with `RegexResolver` and `TreeSitterResolver` feature flag backends.
- **Criterion Benchmark Suite**: Scaling benchmarks for PPT tournament and pruning efficiency.
- **Python 3.10 Compatibility**: Seamless polyfills for typing and lazy litellm imports.

---

## [0.1.0] - 2026-08-20

### Added
- Initial dual-engine CEGAR pipeline with Gemini Creator and DeepSeek Curator.
- FastMCP server implementation for Cursor and Claude Code.
- Isolated subprocess sandbox runner for adversarial test execution.
