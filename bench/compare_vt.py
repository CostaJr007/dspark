import json

for path, label in [
    (r"bench\results\results_1787810974.jsonl", "gpt-4o-mini (19 tasks)"),
    (r"bench\results\results_1787890859.jsonl", "gpt-3.5-turbo (11 tasks)"),
]:
    rows = [json.loads(l) for l in open(path, encoding="utf-8") if l.strip()]
    n = len(rows)
    if n == 0:
        continue

    def pct(k):
        return sum(1 for r in rows if r[k]) / n

    c = sum(r["C_random_expected_pass"] for r in rows) / n
    worst = [r for r in rows if not r["T_ppt_pick_pass"] and r["V_verify_all_pass"]]
    print(f"{label}:")
    print(f"  V first-pass scan  : {pct('V_verify_all_pass')*100:.1f}%  (escolher o 1o draft que passa)")
    print(f"  T PPT pick         : {pct('T_ppt_pick_pass')*100:.1f}%  (binario e soft empatados)")
    print(f"  C random esperado  : {c*100:.1f}%")
    print(f"  D full (escalacao) : {pct('D_full_pass')*100:.1f}%")
    print(f"  PPT errou tendo PASS disponivel: {len(worst)}/{n} tasks -> {[r['task_id'] for r in worst]}")
    print()
