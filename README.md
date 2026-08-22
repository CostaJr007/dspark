# DSpark

Native Rust **dual-engine** for code: a **creator** drafts, a **curator** from a different model family audits I/O contracts. Roles, not vendors.

Default pair (what this repo actually runs with OpenAI + DeepSeek keys):

- **creator** `gpt-4o-mini`
- **curator** `deepseek-v4-pro`

Config lives in `~/.dspark/pair.toml`. This project does not replace or modify `grok.exe`.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![MCP](https://img.shields.io/badge/MCP-Compatible-green.svg)](https://modelcontextprotocol.io/)

[Português](README.pt-BR.md)

---

## Binaries (do not mix them up)

| Binary | What it is |
|---|---|
| `dspark-cli` | Fullscreen TUI (Grok Build rebrand, Apache-2.0). Installed separately. Dual-engine is the default workflow (`/pair`, tool `dspark_curate`). |
| `dspark` | This crate: pipeline CLI (`run`, `audit`, `refine`, `arbitrate`, REPL). |
| `dspark-bench` | Optional HumanEval / HumanEval+ runner. |

`cargo install --path .` from **this** repo installs `dspark` and `dspark-bench` only. It will **not** overwrite `dspark-cli`.

---

## How it works

```
spec / task
    → web research (optional)
    → creator draft
    → curator I/O audit (spec, contracts, doctest oracle — not the agent's self-score)
    → refine if needed
    → re-audit
```

Search is live: Tavily when `TAVILY_API_KEY` is set, otherwise DuckDuckGo HTML. Empty results stay empty — no fake hits.

---

## Install (engine)

```bash
git clone https://github.com/CostaJr007/dspark.git
cd dspark
cargo install --path . --force
```

```bash
dspark --help
dspark pair
```

### Environment

```bash
export OPENAI_API_KEY="sk-..."            # creator default
export DEEPSEEK_API_KEY="sk-..."          # curator default
export GEMINI_API_KEY="..."               # only if the creator is Gemini
export TAVILY_API_KEY="..."               # optional ranked search
export DSPARK_CREATOR="gpt-4o-mini"       # overrides pair.toml
export DSPARK_CURATOR="deepseek-v4-pro"
```

PowerShell:

```powershell
$env:OPENAI_API_KEY="sk-..."
$env:DEEPSEEK_API_KEY="sk-..."
```

### Pair file

Copy [dspark.toml.example](dspark.toml.example) to `%USERPROFILE%\.dspark\pair.toml` (or `~/.dspark/pair.toml`):

```toml
creator = "gpt-4o-mini"
curator = "deepseek-v4-pro"
research = true
```

`dspark pair` prints the active pair and warns if both roles are the same model.

---

## Commands

No-args starts the interactive REPL with the saved pair:

```bash
dspark
dspark interactive --theme bloomberg
```

One-shot and utilities:

```bash
dspark pair
dspark run "LeetCode 1 Two Sum in Python" --lang python --no-research --out two_sum.py
dspark audit two_sum.py --spec "Return indices of two numbers that add to target" --lang python
dspark refine two_sum.py --spec "Handle duplicates; O(n) hashmap" --in-place --lang python
dspark search "tokio spawn_blocking vs spawn"
dspark fetch https://docs.rs/tokio
dspark local
```

REPL slash commands: `/search`, `/fetch`, `/files`, `/read`, `/sh`, `/audit`, `/refine`, `/local`, `/models`, `/theme bloomberg|cyan|matrix`.

---

## TUI (`dspark-cli`)

The fullscreen UI is the daily driver. Default model is the creator; `/pair` sets creator + curator (`fork_secondary_model`). After edits the agent is reminded to call `dspark_curate`.

Do not `cargo install` this engine crate over an existing `dspark-cli.exe` TUI (~300 MB).

---

## MCP

```json
{
  "mcpServers": {
    "dspark": {
      "command": "dspark",
      "args": ["mcp"],
      "env": {
        "DEEPSEEK_API_KEY": "your-key",
        "OPENAI_API_KEY": "your-key"
      }
    }
  }
}
```

Tools: `dspark_audit_code`, `dspark_refine_code`, `dspark_arbitrate`.

---

## Rust library

```rust
use dspark::DeepSeekCurator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let curator = DeepSeekCurator::new()?;
    let verdict = curator
        .audit(
            "fn divide_chunks(l: &[u8], n: usize) {}",
            "Chunk a list into batches of size n. Handle n == 0 safely.",
            Some("rust"),
        )
        .await?;
    println!("Verdict: {} (Score: {}/100)", verdict.verdict, verdict.score);
    Ok(())
}
```

```bash
cargo run --example simple_audit
cargo run --example arbitrate_candidates
```

The `dspark/` Python package is a compatibility SDK (`pip install -e .`). It does **not** install a console script that shadows `dspark` or `dspark-cli`. Use `python -m dspark.cli` only for the legacy entrypoint.
