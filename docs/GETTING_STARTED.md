# 🚀 Getting Started with DSpark

This guide walks you through installing, configuring, and running your first speculative dual-engine verification with DSpark.

---

## 1. Prerequisites

- **Rust toolchain** (1.75+):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Python** (3.10+):
  ```bash
  python --version
  ```
- **API Keys**: DeepSeek, OpenAI, or Google Gemini API Key.

---

## 2. Installation

### Option A: Install from Source (Recommended)

```bash
git clone https://github.com/CostaJr007/dspark.git
cd dspark

# Install the Rust CLI
cargo install --path crates/dspark-core --force

# Install the Python SDK
pip install -e .
```

### Option B: With Tree-Sitter AST Feature

```bash
cargo install --path crates/dspark-core --features tree-sitter-ast --force
```

---

## 3. Configuration & API Keys

Set your API keys in your environment:

```bash
# Linux / macOS
export DEEPSEEK_API_KEY="sk-..."
export OPENAI_API_KEY="sk-..."
export GEMINI_API_KEY="AIza..."

# Windows PowerShell
$env:DEEPSEEK_API_KEY="sk-..."
$env:OPENAI_API_KEY="sk-..."
$env:GEMINI_API_KEY="AIza..."
```

Configure your default Creator / Curator model pair:

```bash
dspark pair --creator gpt-4o-mini --curator deepseek-chat
```

---

## 4. First Execution

### Speculative Multi-Trajectory Run

```bash
dspark run "Implement a thread-safe Token Bucket rate limiter in Python" \
           --speculative \
           --trajectories 3 \
           --pivots 2 \
           --out rate_limiter.py
```

### Expected Output

```text
🚀 Speculative Orchestration: Drafter=gpt-4o-mini Verifier=deepseek-chat (N=3, Pivots=2)

[1/4] Generating 3 speculative trajectories in parallel...
  ✓ Obtained 3 structurally valid AST trajectories

[2/4] Estimating local entropy & confidence scores...
  Trajectory #1: 1/3 blocks require verification
  Trajectory #2: 0/3 blocks require verification
  Trajectory #3: 1/3 blocks require verification

[3/4] Running Cost-Aware Scheduler & pruning...
  ✓ Scheduled 2 verifications (Pruned 7 blocks, Est. API cost: $0.0040, Acceptance: 92.5%)

[4/4] Conducting Probabilistic Pivot Tournament (k=2 pivots)...

=== 🏆 SPECULATIVE ORCHESTRATION WINNER ===
Winning Trajectory: Trajectory #2
Total Tournament Comparisons: 5
Final verified code written to rate_limiter.py
```

---

## 5. Integrating with IDEs (MCP Server)

Start the FastMCP server:

```bash
dspark-mcp
```

In Cursor or Claude Code, add the MCP tool endpoint:
- **Command**: `dspark-mcp`
- **Exposed Tools**: `dspark_audit`, `dspark_refine`, `dspark_generate_contracts`, `dspark_run_cegar`.
