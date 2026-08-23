# 🔌 Example: IDE Integration via FastMCP

DSpark exposes a native Model Context Protocol (MCP) server that connects with Cursor, Windsurf, Claude Code, Roo Code, and Google Antigravity.

---

## 1. Starting the Server

```bash
dspark-mcp
```

---

## 2. Cursor / Claude Code Settings

Add to your `mcp.json` or IDE MCP configuration:

```json
{
  "mcpServers": {
    "dspark": {
      "command": "dspark-mcp",
      "env": {
        "DEEPSEEK_API_KEY": "sk-...",
        "OPENAI_API_KEY": "sk-..."
      }
    }
  }
}
```

---

## 3. Exposed MCP Tools

- `dspark_audit_code`: Performs independent adversarial verification on a code snippet against formal I/O contracts.
- `dspark_refine_code`: Refines failing code using curator counterexamples.
- `dspark_generate_contracts`: Synthesizes formal preconditions, postconditions, and invariants from docstrings.
- `dspark_run_cegar`: Executes a full multi-turn CEGAR verification loop inside an isolated sandbox.
