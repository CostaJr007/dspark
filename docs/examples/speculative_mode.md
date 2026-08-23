# ⚡ Example: Speculative Multi-Trajectory Mode

Speculative mode leverages parallel trajectory drafting, AST topological sorting, local entropy pruning, and $O(Nk)$ Probabilistic Pivot Tournaments.

---

## 1. Running Speculative Generation

Execute multi-trajectory generation with 4 candidate drafts and 2 tournament pivots:

```bash
dspark run "Implement Dijkstra's shortest path algorithm with priority queue in Rust" \
           --speculative \
           --trajectories 4 \
           --pivots 2 \
           --lang rust \
           --out dijkstra.rs
```

---

## 2. What Happens Under the Hood

1. **Parallel Rayon / Tokio Drafting**: 4 diverse solutions are generated with temperatures $T \in [0.2, 0.65]$.
2. **AST Dependency Ordering**: Analyzes dependencies using `petgraph` to order helper structs and functions.
3. **Local Confidence Analysis**: Evaluates cycle safety and cyclomatic entropy on CPU.
4. **Pivot Tournament**: Conducts an $O(Nk)$ tournament, selecting the highest-quality candidate.
5. **CEGAR Sandbox Verification**: Executes the final code in an isolated sandbox.
