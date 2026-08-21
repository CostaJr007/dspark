# DSpark CLI

Native Rust CLI for **dual-engine** code work: a **creator** drafts, a **curator** from a different model family audits I/O contracts. Binary name: `dspark-cli`.

Creator and curator are **roles**, not vendors. You pick both. Typical pairing is a Gemini-class draft plus a DeepSeek-class curator so the reviewer is not the same family that wrote the code (same-model self-review is confirmation-biased).

Config lives in `~/.dspark/config.toml`. This project does not replace or modify `grok` on your machine.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![MCP](https://img.shields.io/badge/MCP-Compatible-green.svg)](https://modelcontextprotocol.io/)

[Português](README.pt-BR.md)

---

## How it works

```
spec / task
    → web research (optional)
    → creator draft
    → curator I/O audit (spec, contracts, errors — not the agent's self-score)
    → refine if needed
    → re-audit
```

The curator is LLM-as-a-Verifier: it scores preconditions, postconditions, edge cases, complexity, and resource safety. It must not trust the creator's own assessment. After a refine pass, it audits again.

Search is live: Tavily when `TAVILY_API_KEY` is set, otherwise DuckDuckGo HTML. Empty results stay empty — no fake hits.

---

## Install

```bash
git clone https://github.com/CostaJr007/dspark.git
cd dspark
cargo install --path .
```

That installs `dspark-cli` (typically `%USERPROFILE%\.cargo\bin` on Windows, `~/.cargo/bin` elsewhere).

```bash
dspark-cli --help
dspark-cli pair
```

### Environment

```bash
export DEEPSEEK_API_KEY="sk-..."          # curator default
export GEMINI_API_KEY="..."               # creator default (optional)
export OPENAI_API_KEY="..."               # if you pick an OpenAI-family role
export TAVILY_API_KEY="..."               # optional ranked search
export DSPARK_CREATOR="gemini-3.7-flash"  # overrides config
export DSPARK_CURATOR="deepseek-v4-pro"
```

PowerShell:

```powershell
$env:DEEPSEEK_API_KEY="sk-..."
```

### Pair config

Copy [dspark.toml.example](dspark.toml.example) to `%USERPROFILE%\.dspark\config.toml` (or `~/.dspark/config.toml`):

```toml
creator = "gemini-3.7-flash"
curator = "deepseek-v4-pro"
research = true
```

`dspark-cli pair` prints the active pair and warns if both roles are the same model.

---

## Commands

No-args starts the interactive REPL with the saved pair:

```bash
dspark-cli
dspark-cli interactive --generator gemini-3.7-flash --curator deepseek-v4-pro --theme bloomberg
```

One-shot and utilities:

```bash
dspark-cli pair
dspark-cli search "tokio spawn_blocking vs spawn"
dspark-cli search --deep "FastAPI background tasks"
dspark-cli fetch https://docs.rs/tokio
dspark-cli audit src/search.rs --spec "Binary search, O(log N), empty list is valid"
dspark-cli refine src/lru.rs --spec "O(1) get/put, bounded capacity" --in-place
dspark-cli arbitrate a.rs b.rs --spec "Lock-free queue"
dspark-cli run "Bounded LRU cache in Rust" --lang rust --out lru.rs
dspark-cli local
dspark-cli "Refactor auth.rs to bcrypt and cover empty password"
```

REPL slash commands: `/search`, `/fetch`, `/files`, `/read`, `/sh`, `/audit`, `/refine`, `/local`, `/models`, `/theme bloomberg|cyan|matrix`.

---

## MCP

```json
{
  "mcpServers": {
    "dspark": {
      "command": "dspark-cli",
      "args": ["mcp"],
      "env": {
        "DEEPSEEK_API_KEY": "your-key"
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

---

## Optional Python SDK

The `dspark/` package is a compatibility SDK (`pip install -e .`). It does **not** install a `dspark` console script, so it cannot shadow `dspark-cli`. Use `python -m dspark.cli` only if you want the legacy Python entrypoint.

---

## License

MIT — [Adeilson Costa](https://github.com/CostaJr007).
