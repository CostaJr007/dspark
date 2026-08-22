---
name: dspark
description: Dual-engine creator/curator. A quality creator drafts; a different-family curator audits I/O contracts. Triggers: dspark, curator, dual-engine, /pair.
---

# DSpark dual-engine

Creator and curator are roles. Typical pairing: Gemini-class draft + DeepSeek-class I/O curator.

- Dual-engine is automatic: after a code edit the runtime runs the curator. Do not ask the user to curate.
- `/pair` only chooses creator and curator models (`~/.dspark/pair.toml`).
- If the curator reminder says refined code was applied, re-read the file.
- Do not trust the creator's self-score.
