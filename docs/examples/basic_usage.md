# 💡 Example: Basic Usage & Code Generation

This guide shows how to use DSpark to generate, audit, and refine code in single-trajectory standard mode.

---

## 1. Defining a Specification

Create a markdown or text specification file (e.g. `task_lru.md`):

```markdown
# Problem: LRU Cache Implementation

Write an LRU (Least Recently Used) cache in Python with the following API:
- `__init__(capacity: int)`: Initializes cache with positive capacity.
- `get(key: int) -> int`: Returns value or -1 if not found.
- `put(key: int, value: int) -> None`: Inserts or updates key. Evicts least recently used item when capacity is exceeded.

Constraints:
- All operations must run in O(1) average time complexity.
```

---

## 2. Generating with Dual-Engine Verification

Run DSpark with your configured Creator and Curator:

```bash
dspark run "Implement LRU Cache as specified in task_lru.md" --out lru_cache.py
```

---

## 3. Auditing Existing Code

If you already have code and want DeepSeek to audit it against a contract:

```bash
dspark audit --file lru_cache.py --spec task_lru.md --logprobs
```
