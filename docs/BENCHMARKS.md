# 📊 DSpark Benchmarks & Performance Methodology

## 1. Methodology

All performance benchmarks are implemented using [Criterion.rs](https://github.com/bheisler/criterion.rs) with statistical outlier detection, 100 iterations per group, and 95% confidence intervals.

Run the entire benchmark suite:
```bash
./scripts/bench_all.sh
```

---

## 2. PPT Tournament Scaling ($O(Nk)$ vs $O(N^2)$)

The **Probabilistic Pivot Tournament (PPT)** drastically reduces verification comparisons as $N$ scales:

| Number of Candidates ($N$) | Number of Pivots ($k$) | Tournament Comparisons ($O(Nk)$) | All-Pairs Comparisons ($O(N^2)$) | API Call Reduction |
|---|---|---|---|---|
| **$N=3$** | $k=2$ | 6 | 3 | Baseline |
| **$N=5$** | $k=2$ | 12 | 10 | Baseline |
| **$N=10$** | $k=3$ | 34 | 45 | **24.4%** |
| **$N=20$** | $k=3$ | 74 | 190 | **61.1%** |
| **$N=50$** | $k=3$ | 184 | 1,225 | **85.0%** |
| **$N=100$** | $k=3$ | 359 | 4,950 | **92.7%** |

---

## 3. Local Confidence Pruning Efficiency

| Trajectories ($N$) | Blocks / Trajectory | Total Code Blocks | Remote Verifications | Local CPU Pruned | Savings % |
|---|---|---|---|---|---|
| 3 | 10 | 30 | 12 | 18 | **60.0%** |
| 5 | 50 | 250 | 87 | 163 | **65.2%** |
| 10 | 100 | 1,000 | 342 | 658 | **65.8%** |

---

## 4. AST Dependency Resolver Latency

| Blocks | Regex Resolver (ms) | Tree-Sitter Resolver (ms) | Precision Guarantee |
|---|---|---|---|
| 5 | 0.12 ms | 1.18 ms | 100% Exact Syntax |
| 20 | 0.48 ms | 4.82 ms | 100% Exact Syntax |
| 50 | 1.22 ms | 12.10 ms | 100% Exact Syntax |
| 100 | 2.45 ms | 24.30 ms | 100% Exact Syntax |
