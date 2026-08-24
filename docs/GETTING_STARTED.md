# 🚀 Getting Started with DSpark

This guide walks you through installing, configuring, and running your first speculative dual-engine verification with DSpark, as well as integrating it into your favorite IDE via MCP.

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
- **API Keys**: DeepSeek API Key, OpenAI API Key, or Google Gemini API Key.
- *(Optional)* **Ollama / vLLM**: For running 100% local drafting models at zero cost.

---

## 2. Installation

### Option A: Install from Source (Recommended)

```bash
git clone https://github.com/CostaJr007/dspark.git
cd dspark

# Install the Rust CLI (Fast regex AST backend)
cargo install --path crates/dspark-core --force

# Install the Python SDK & CLI
pip install -e .
```

### Option B: Install with Tree-Sitter AST Support

```bash
cargo install --path crates/dspark-core --features tree-sitter-ast --force
```

---

## 3. Environment Configuration

Set your API keys in your environment or in a `.env` file:

### Linux / macOS
```bash
export DEEPSEEK_API_KEY="sk-..."
export OPENAI_API_KEY="sk-..."
export GEMINI_API_KEY="AIza..."
```

### Windows PowerShell
```powershell
$env:DEEPSEEK_API_KEY="sk-..."
$env:OPENAI_API_KEY="sk-..."
$env:GEMINI_API_KEY="AIza..."
```

### Configure Default Pair
```bash
# Pair a cheap drafter with a flagship curator
dspark pair --creator gpt-4o-mini --curator deepseek-chat
```

---

## 4. First Execution

### Multi-Trajectory Speculative Run

Generate code using 3 concurrent trajectories and tournament ranking:

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

## 5. Integrating with IDEs via MCP

DSpark exposes a standardized **Model Context Protocol (FastMCP)** server.

### A. Cursor (`~/.cursor/mcp.json` or project `mcp.json`)
```json
{
  "mcpServers": {
    "dspark": {
      "command": "python",
      "args": ["-m", "dspark.mcp.server"],
      "cwd": "C:/Users/adeil/dspark",
      "env": {
        "DEEPSEEK_API_KEY": "your-deepseek-key",
        "OPENAI_API_KEY": "your-openai-key"
      }
    }
  }
}
```

### B. Claude Code & Claude Desktop (`claude_desktop_config.json`)
```json
{
  "mcpServers": {
    "dspark-dual-engine": {
      "command": "python",
      "args": ["-m", "dspark.mcp.server"],
      "cwd": "C:/Users/adeil/dspark",
      "env": {
        "DEEPSEEK_API_KEY": "your-deepseek-key"
      }
    }
  }
}
```

### C. Antigravity (AGY)
Antigravity automatically discovers DSpark when placed in:
`~/.gemini/config/plugins/dspark/`

### D. Windsurf & Roo Code
Add `dspark` in your IDE MCP settings with command `python -m dspark.mcp.server`.

---

## 6. Exposed MCP Tools

Once configured, your AI assistant in Cursor or Claude will automatically have access to:

| Tool | Parameters | Description |
| :--- | :--- | :--- |
| `dspark_audit` | `code`, `contracts_json` | Runs adversary contract tests in an isolated sandbox, returning pass/fail + counterexamples. |
| `dspark_refine` | `code`, `counterexample`, `task` | Applies surgical 1-shot repair using epistemically isolated DeepSeek Flagship. |
| `dspark_verify_pipeline` | `task_description` | Runs the entire speculative drafting + tournament + sandbox verification pipeline. |
