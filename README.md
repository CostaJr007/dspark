# DSpark

DSpark is a **dual-engine** coding system: one model **creates**, another model **curates**.

A single LLM writing and then “reviewing itself” is confirmation-biased. DSpark splits the job into two **roles** (not vendors):

| Role | Job |
|---|---|
| **Creator** | Drafts the implementation from the spec. Fast / cheap is fine. |
| **Curator** | Independent **LLM-as-a-Verifier**. Scores I/O contracts (preconditions, postconditions, edge cases), synthesizes counter-examples, and may rewrite the draft. Must be a **different model family** than the creator. |

You only choose *which* two models fill those roles. Curation is the default workflow — you do not ask for it.

Default pair on this machine / in `~/.dspark/pair.toml`:

```toml
creator = "gpt-4o-mini"
curator = "deepseek-v4-pro"
```

[Português](README.pt-BR.md) · License: [MIT](LICENSE)

---

## Two repositories (do not mix them up)

| Repo | Binary | What it is |
|---|---|---|
| **[CostaJr007/dspark-cli](https://github.com/CostaJr007/dspark-cli)** | `dspark-cli` | **Daily driver.** Fullscreen TUI in a real repo. Creator writes files; after each code edit the curator runs automatically and may apply a refine. |
| **This repo (`dspark`)** | `dspark` | **Engine.** Pipeline CLI, Rust library, and MCP server used by the TUI, by Agy, and by scripts. |

Work on a project:

```powershell
cd your-project
dspark-cli
```

Ask for the feature. Do not say “curate”. `/pair` only changes the two models.

---

## What the engine actually does

```
specification (what the code must do)
        │
        ▼
   creator draft          ← GPT / Gemini / local / whoever you set
        │
        ▼
   curator audit          ← DeepSeek (or another family)
        │                   verdict: APPROVED | NEEDS_REVISION | REJECTED
        │                   score 0–100, critical issues, counter-examples
        ▼
   refine if needed       ← same curator rewrites against the spec + feedback
        │
        ▼
   re-audit               ← do not ship on a self-score
```

The curator does **not** trust “tests passed” narration from the creator. It checks:

- **Specification** — stated requirements are implemented
- **I/O contract** — empty/null, types, documented errors, doctest `>>>` examples, encode/decode roundtrips when both helpers exist
- **Errors** — no silent swallow of the contract

`APPROVED` with score 100 is forbidden if those examples were not checked.

---

## Install this engine

```bash
git clone https://github.com/CostaJr007/dspark.git
cd dspark
cargo install --path . --force
```

Installs **`dspark` only**. It will not overwrite `dspark-cli`.

```bash
export OPENAI_API_KEY="..."       # if the creator is OpenAI-class
export DEEPSEEK_API_KEY="..."     # curator (required for audit/refine/MCP)
export GEMINI_API_KEY="..."       # only if the creator is Gemini via this CLI
export DSPARK_CREATOR="gpt-4o-mini"
export DSPARK_CURATOR="deepseek-v4-pro"
```

Copy [dspark.toml.example](dspark.toml.example) to `~/.dspark/pair.toml`.

---

## Commands

```bash
dspark pair                          # print active creator/curator
dspark run "Bounded LRU in Rust" --lang rust --no-research --out lru.rs
dspark audit lru.rs --spec "O(1) get/put, bounded capacity" --lang rust
dspark refine lru.rs --spec "O(1) get/put, bounded capacity" --in-place --lang rust
dspark arbitrate a.rs b.rs --spec "Lock-free queue"
dspark search "tokio spawn_blocking vs spawn"
dspark                       # REPL (metacognitive agent + verify_with_curator)
```

`dspark run` is the full pipeline (generate → audit → refine → re-audit).  
`dspark audit` / `refine` are the curator alone, when you already have a file.

---

## MCP (Agy, editors, other agents)

This repo exposes the curator as MCP tools so **Gemini (or any host)** can stay the creator and still call DeepSeek as verifier:

- `dspark_audit_code`
- `dspark_refine_code`
- `dspark_arbitrate`

```json
{
  "mcpServers": {
    "dspark": {
      "command": "dspark",
      "args": ["mcp"]
    }
  }
}
```

On this PC, Agy is already wired that way (`agy mcp list` → `dspark`). Session model = Gemini Flash; curator = `deepseek-v4-pro` from `pair.toml`. Switch to flash with `curator = "deepseek-v4-flash"`.

---

## Library

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

Live search (optional): Tavily if `TAVILY_API_KEY` is set, otherwise DuckDuckGo HTML. Empty results stay empty — no invented hits.
