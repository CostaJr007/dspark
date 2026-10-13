#!/usr/bin/env python3
"""Verdict analysis for a weak-vs-strong live pilot.

Reads a results jsonl, prints: pass rates per config, token totals per tier,
and projected $ under two paid rate cards (OpenAI-ish, DeepSeek-ish),
plus cost-per-success per strategy. Usage: python bench/verdict.py <results.jsonl>
"""
import json
import sys

OPENAI_CARD = {  # $/token in/out
    "weak": (0.15e-6, 0.60e-6),    # gpt-4o-mini class
    "strong": (2.50e-6, 10.00e-6),  # gpt-4o class
}
DEEPSEEK_CARD = {
    "weak": (0.27e-6, 1.10e-6),    # deepseek-chat class
    "strong": (0.55e-6, 2.19e-6),  # deepseek-reasoner class
}


def main(path: str) -> None:
    rows = [json.loads(l) for l in open(path) if l.strip()]
    n = len(rows)
    print(f"tasks: {n}")

    def rate(k):
        return sum(1 for r in rows if r.get(k)) / n

    print(f"cheap-only : {rate('B_cheap_pass'):.1%}")
    print(f"flagship-only: {rate('A_flagship_pass'):.1%}")
    print(f"PPT pick   : {rate('T_ppt_pick_pass'):.1%}")
    print(f"FULL tiered: {rate('D_full_pass'):.1%}")
    print(f"escalations: {sum(1 for r in rows if r.get('escalated'))}")

    # token totals: attribute spend by tier from the final snapshot proportions
    toks = {}
    for r in rows:
        for m, (i, o) in r.get("token_snapshot", {}).items():
            t = toks.get(m, [0, 0])
            toks[m] = [t[0] + i, t[1] + o]
    weak_tok = [0, 0]
    strong_tok = [0, 0]
    STRONG = {"openai/gpt-oss-120b", "gpt-4o", "deepseek-chat", "deepseek-reasoner",
              "deepseek-v4-flash", "qwen/qwen3.8-27b", "ollama:qwen2.5-coder:7b"}
    for m, (i, o) in toks.items():
        tgt = strong_tok if m in STRONG else weak_tok
        tgt[0] += i
        tgt[1] += o
    print(f"weak-tier tokens  in/out : {weak_tok[0]}/{weak_tok[1]}")
    print(f"strong-tier tokens in/out: {strong_tok[0]}/{strong_tok[1]}")

    for name, card in (("OpenAI-card", OPENAI_CARD), ("DeepSeek-card", DEEPSEEK_CARD)):
        tiered = (weak_tok[0] * card["weak"][0] + weak_tok[1] * card["weak"][1]
                  + strong_tok[0] * card["strong"][0] + strong_tok[1] * card["strong"][1])
        # flagship-only counterfactual: one strong generation ≈ strong tokens minus judging;
        # upper-bound it with the observed strong spend (conservative for tiered).
        print(f"{name}: tiered projected ${tiered:.4f} total (${tiered / n:.4f}/task)")


if __name__ == "__main__":
    main(sys.argv[1])
