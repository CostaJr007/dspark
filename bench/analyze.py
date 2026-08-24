import json, statistics, random, sys

rows = [json.loads(l) for l in open(r"bench\results\results_1787497146.jsonl", encoding="utf-8")]
print("n tasks:", len(rows))
n = len(rows)

def pct(k):
    return sum(1 for r in rows if r[k]) / n

def c_exp():
    return statistics.fmean(r["C_random_expected_pass"] for r in rows)

def wilson(p, n, z=1.96):
    if n == 0:
        return (0, 0)
    den = 1 + z * z / n
    c = (p + z * z / (2 * n)) / den
    h = z * ((p * (1 - p) / n + z * z / (4 * n * n)) ** 0.5) / den
    return (max(0, c - h), min(1, c + h))

for k, name in [("B_cheap_pass", "cheap-only"), ("A_flagship_pass", "flagship-only"),
                ("V_verify_all_pass", "verify-all"), ("T_ppt_pick_pass", "PPT pick"),
                ("D_full_pass", "FULL tiered")]:
    p = pct(k)
    lo, hi = wilson(p, n)
    print(f"{name:<20} {p*100:5.1f}%  [{lo*100:.1f}-{hi*100:.1f}]")
c = c_exp()
lo, hi = wilson(c, n)
print(f"{'best-of-3 RANDOM':<20} {c*100:5.1f}%  [{lo*100:.1f}-{hi*100:.1f}]")

d1 = [r["T_ppt_pick_pass"] - r["C_random_expected_pass"] for r in rows]
m1 = statistics.fmean(d1)
rng = random.Random(7)
boots = []
for _ in range(2000):
    s = [d1[rng.randrange(n)] for _ in range(n)]
    boots.append(statistics.fmean(s))
boots.sort()
print(f"Q1 PPT vs RANDOM: {m1:+.1%} pts [{boots[49]:+.1%},{boots[1949]:+.1%}]")

d2 = [r["D_full_pass"] - r["T_ppt_pick_pass"] for r in rows]
m2 = statistics.fmean(d2)
esc = [r for r in rows if r["escalated"]]
prec = sum(1 for r in esc if not r["T_ppt_pick_pass"]) / len(esc) if esc else float("nan")
fix = sum(1 for r in esc if r["refined_pass"])
print(f"Q2 +v4-flash: {m2:+.1%} pts | escalations={len(esc)} precision={prec:.0%} fixes={fix}/{len(esc)}")
print("spend:", rows[-1]["budget_snapshot"])

# quais tarefas o flagship refina com sucesso (cases de escalonamento)
print("\ncasos escalados (flagship consertou?):")
for r in esc:
    if r["kind"] == "creation":
        print(f"  {r['task_id']:<25} winner={r['ppt_winner_idx']} refined={'PASS' if r['refined_pass'] else 'FAIL'}")
    else:
        print(f"  {r['task_id']:<25} refined={'PASS' if r['refined_pass'] else 'FAIL'}")
