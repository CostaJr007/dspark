---
name: dspark
description: Dual-engine creator/curator. A quality creator drafts; a different-family curator audits I/O contracts. Activate for algorithms, mission-critical logic, refactoring, or explicit I/O verification.
---

# DSpark: dual-engine curator

Prefer the native binary `dspark-cli`. Creator and curator are roles you choose (default pairing: Gemini-class draft + DeepSeek-class I/O curator).

## When to activate

- Complex algorithms (state machines, concurrency, parsers, financial math).
- Mission-critical endpoints with strict I/O typing.
- The user asks to curate, arbitrate, or validate edge cases.

## How to curate

```powershell
dspark-cli audit path/to/file.py --spec "Expected behavior, edge cases, and I/O contracts"
dspark-cli refine path/to/file.py --spec "Requirements" --in-place
```

If the binary is not on PATH, fall back to:

```powershell
python skill/dspark/scripts/curate.py path/to/file.py --spec "Requirements"
python skill/dspark/scripts/curate.py path/to/file.py --spec "Requirements" --refine
```
