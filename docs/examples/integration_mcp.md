# MCP Integration

DSpark exposes MCP servers over stdio. There are two tool surfaces — use the one
matching your launcher (do not mix tool names between them).

## Surface A — `dspark_audit_code` / `dspark_refine_code` / `dspark_arbitrate`

Provided by both:

- Rust binary (recommended, no Python runtime needed): `dspark mcp`
- Python CLI: `dspark mcp` (runs `dspark.mcp_server`, same tools)

Verify a server responds (line-delimited JSON over stdio):

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | dspark mcp
```

### Cursor / Windsurf (`mcp.json`)

```json
{
  "mcpServers": {
    "dspark": {
      "command": "dspark",
      "args": ["mcp"],
      "env": {
        "DEEPSEEK_API_KEY": "<your-deepseek-key>",
        "OPENAI_API_KEY": "<your-openai-key>"
      }
    }
  }
}
```

### Claude Desktop / Claude Code

Same shape under `mcpServers` in `claude_desktop_config.json` (Desktop) or
`~/.claude.json` (Code).

### Antigravity / AGY CLI (`~/.gemini/config/plugins/dspark/mcp_config.json`)

```json
{
  "mcpServers": {
    "dspark": {
      "command": "dspark",
      "args": ["mcp"],
      "env": {
        "DSPARK_CURATOR": "gemini:gemini-2.5-flash"
      }
    }
  }
}
```

## Surface B — Python FastMCP (`dspark-mcp`)

Installed by `pip install -e .` (entry point `dspark.mcp.server`, needs the
`mcp` package). Different tool names:

- `dspark_audit`, `dspark_refine`, `dspark_generate_contracts`, `dspark_run_cegar`

```json
{
  "mcpServers": {
    "dspark": {
      "command": "dspark-mcp"
    }
  }
}
```

## Tool reference (Surface A)

- `dspark_audit_code`: audits candidate code against I/O contracts, returns
  `APPROVED` / `NEEDS_REVISION` with counter-examples.
- `dspark_refine_code`: 1-shot CEGAR repair of failing code from curator feedback.
- `dspark_arbitrate`: Probabilistic Pivot Tournament over N candidates.
