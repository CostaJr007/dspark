# ⌨️ DSpark CLI Reference

Complete reference for all commands, arguments, and options in the `dspark` CLI.

---

## Global Usage

```bash
dspark [COMMAND] [OPTIONS]
```

### Options
- `-h, --help`: Print command line help.
- `-V, --version`: Print version information.

---

## Commands

### 1. `dspark run`
Runs the dual-model code generation, curation, and speculative execution pipeline.

```bash
dspark run [OPTIONS] <PROMPT>
```

#### Arguments & Flags:
| Argument / Flag | Type | Default | Description |
|---|---|---|---|
| `<PROMPT>` | `String` | *(Required)* | Natural language specification or prompt |
| `-d, --draft` | `String` | `None` | Path to an existing draft file to audit/refine |
| `-l, --lang` | `String` | `"python"` | Target programming language |
| `-o, --out` | `String` | `None` | Output path to save the final verified code |
| `-g, --generator` | `String` | Configured Creator | Override Creator model (e.g. `gpt-4o-mini`, `gemini-2.5-flash`) |
| `-c, --curator` | `String` | Configured Curator | Override Curator model (e.g. `deepseek-chat`, `deepseek-v4-pro`) |
| `--no-research` | `bool` | `false` | Skip live DuckDuckGo web documentation research |
| `--speculative` | `bool` | `false` | Enable speculative multi-trajectory generation & PPT tournament |
| `--trajectories` | `usize` | `3` | Number of parallel speculative trajectories ($N$) |
| `--pivots` | `usize` | `2` | Number of tournament pivots ($k$) |

---

### 2. `dspark pair`
Manages the active Creator and Curator model pair.

```bash
# View active pair
dspark pair

# Configure new pair
dspark pair --creator <CREATOR_MODEL> --curator <CURATOR_MODEL>
```

---

### 3. `dspark search`
Performs live web research and extracts documentation in clean markdown.

```bash
dspark search "PySide6 QThread worker pattern" --max 3
```

---

### 4. `dspark-mcp`
Launches the Model Context Protocol (MCP) server over stdio for IDE integration.

```bash
dspark-mcp
```
