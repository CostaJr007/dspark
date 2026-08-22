# DSpark

Dual-engine coding: a **creator** drafts, a **curator** from a different model family audits I/O contracts. Roles, not vendors.

Default pair:

| Role | Model |
|---|---|
| creator | `gpt-4o-mini` |
| curator | `deepseek-v4-pro` |

Pair file: `~/.dspark/pair.toml`. You pick the two models. Curation is the default — you do not ask for it.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)

[Português](README.pt-BR.md)

---

## What to run

| Command | Use |
|---|---|
| **`dspark-cli`** | Daily driver. Fullscreen TUI in a repo. Creator writes; curator runs after each code edit. |
| **`dspark`** | This crate. Pipeline and library: `run`, `audit`, `refine`, `arbitrate`, REPL, MCP. |

TUI source: [CostaJr007/dspark-app](https://github.com/CostaJr007/dspark-app).

```powershell
cd your-project
dspark-cli
```

`/pair` only changes which models fill the two roles.

---

## This repository (`dspark`)

```
spec
  → creator draft
  → curator I/O audit (contracts, doctest oracle — not the author's self-score)
  → refine if needed
  → re-audit
```

```bash
git clone https://github.com/CostaJr007/dspark.git
cd dspark
cargo install --path . --force
```

That installs **`dspark` only**. It does not overwrite `dspark-cli`.

```bash
dspark pair
dspark run "Bounded LRU cache in Rust" --lang rust --no-research --out lru.rs
dspark audit lru.rs --spec "O(1) get/put, bounded capacity" --lang rust
dspark refine lru.rs --spec "O(1) get/put, bounded capacity" --in-place --lang rust
```

### Environment

```bash
export OPENAI_API_KEY="..."      # creator
export DEEPSEEK_API_KEY="..."    # curator
export DSPARK_CREATOR="gpt-4o-mini"
export DSPARK_CURATOR="deepseek-v4-pro"
```

Copy [dspark.toml.example](dspark.toml.example) to `~/.dspark/pair.toml`.

Search is live (Tavily if `TAVILY_API_KEY` is set, otherwise DuckDuckGo HTML). Empty results stay empty.

### Library

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
    println!("{} {}/100", verdict.verdict, verdict.score);
    Ok(())
}
```

```bash
cargo run --example simple_audit
cargo run --example arbitrate_candidates
```

### MCP

```json
{
  "mcpServers": {
    "dspark": {
      "command": "dspark",
      "args": ["mcp"],
      "env": {
        "OPENAI_API_KEY": "your-key",
        "DEEPSEEK_API_KEY": "your-key"
      }
    }
  }
}
```

Tools: `dspark_audit_code`, `dspark_refine_code`, `dspark_arbitrate`.
