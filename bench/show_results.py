#!/usr/bin/env python3
"""Show the real-bench results collected so far (compact table + aggregates)."""
import json
import sys
from pathlib import Path

path = sys.argv[1] if len(sys.argv) > 1 else r"bench\results\results_1787810974.jsonl"
rows = [json.loads(l) for l in Path(path).read_text(encoding="utf-8").splitlines() if l.strip()]

print(f"{len(rows)} tasks em {path}")
print(f"{'task':<14}{'drafts pass':<13}{'T disc':<7}{'T soft':<8}{'div':<6}{'D full':<8}{'esc'}")
for r in rows:
    tbin = "PASS" if r["T_ppt_pick_pass"] else "FAIL"
    tsoft = "PASS" if r.get("T_soft_pick_pass") else "FAIL"
    div = "Y" if r["T_ppt_pick_pass"] != r.get("T_soft_pick_pass") else "-"
    dfull = "PASS" if r["D_full_pass"] else "FAIL"
    print(f"{r['task_id']:<14}{r['n_drafts_passing']}/3{'':<8}{tbin:<7}{tsoft:<8}{div:<6}{dfull:<8}{'y' if r['escalated'] else '-'}")

n = len(rows)
if n == 0:
    sys.exit("sem resultados")

def pct(key, default=None):
    return sum(1 for r in rows if r.get(key, default)) / n

print()
print(f"T disc (PPT discreto 1-5) : {pct('T_ppt_pick_pass') * 100:.1f}%")
print(f"T soft (PPT Bradley-Terry 1-20): {pct('T_soft_pick_pass') * 100:.1f}%")
div = [r for r in rows if r["T_ppt_pick_pass"] != r.get("T_soft_pick_pass")]
print(f"divergencias de pick: {len(div)}/{n}")
for r in div:
    print(f"  {r['task_id']}: bin->{'PASS' if r['T_ppt_pick_pass'] else 'FAIL'} "
          f"soft->{'PASS' if r.get('T_soft_pick_pass') else 'FAIL'} "
          f"(winner bin={r['ppt_winner_idx']}, soft={r.get('ppt_winner_idx_soft')})")
print(f"D full (com escalacao): {pct('D_full_pass') * 100:.1f}%")
print(f"escalacoes: {sum(1 for r in rows if r['escalated'])} | refinamentos que consertaram: "
      f"{sum(1 for r in rows if r.get('refined_pass'))}")
