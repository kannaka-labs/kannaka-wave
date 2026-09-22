#!/usr/bin/env python3
"""E-008 report: the noise floor, the residue, the two AUCs with a paired bootstrap, the
guards, and the decision rule applied exactly as written in
docs/experiments/E-008-faithfulness-as-commutation-with-the-real-operators.md.

    python report.py --results out/results.tsv --labels labels.tsv

labels.tsv: id, labeller_1, labeller_2 with values `endorsed` / `not` (blind to r, s and
the verdict); a probe with a disagreement is dropped, an A1 NONE is not scored.
Standard library only.
"""
from __future__ import annotations

import argparse
import math
import random
import sys
from pathlib import Path

VERDICT_SCORE = {"PACKAGE": 1.0, "ESCALATE": 0.5, "REFUSED": 0.0, "-": 0.0}


def rows(tsv: Path) -> list[dict]:
    lines = tsv.read_text(encoding="utf-8").strip().splitlines()
    head = lines[0].split("\t")
    return [dict(zip(head, l.split("\t"))) for l in lines[1:]]


def disagree(e1: str, e2: str) -> int:
    return 0 if e1 == e2 else 1


def auc(scores: list[float], labels: list[int]) -> float:
    pos = [s for s, l in zip(scores, labels) if l]
    neg = [s for s, l in zip(scores, labels) if not l]
    if not pos or not neg:
        return float("nan")
    wins = sum(1.0 if p > n else 0.5 if p == n else 0.0 for p in pos for n in neg)
    return wins / (len(pos) * len(neg))


def bootstrap(fn, n: int, reps: int, seed: int) -> tuple[float, float]:
    rng = random.Random(seed)
    vals = []
    for _ in range(reps):
        idx = [rng.randrange(n) for _ in range(n)]
        v = fn(idx)
        if not math.isnan(v):
            vals.append(v)
    vals.sort()
    if not vals:
        return float("nan"), float("nan")
    return vals[int(len(vals) * 0.025)], vals[min(len(vals) - 1, int(len(vals) * 0.975))]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--results", type=Path, required=True)
    ap.add_argument("--labels", type=Path)
    ap.add_argument("--reps", type=int, default=1000)
    a = ap.parse_args()
    R = rows(a.results)
    n_all = len(R)
    s = sum(disagree(r["A1_effector"], r["A2_effector"]) for r in R) / n_all
    resid = {r["id"]: disagree(r["A1_effector"], r["B_effector"]) for r in R}
    none_a1 = sum(1 for r in R if r["A1_effector"] == "NONE")
    b_escalated = sum(1 for r in R if r["B_effector"] != "NONE" and r["B_verdict"] != "PACKAGE")
    print(f"E-008 · {n_all} probes")
    print(f"noise floor s (A1 vs A2): {s:.3f}   residue rate r (A1 vs B): {sum(resid.values())/n_all:.3f}")
    print(f"A1 NONE: {none_a1}   B proposals not packaged (admitted on facts, escalated on the action): {b_escalated}")
    guards = []
    if s > 0.15:
        guards.append(f"s = {s:.3f} > 0.15: the voice is too noisy at this temperature; VOID")
    if not a.labels:
        print("no labels: nothing scored")
        for g in guards:
            print("GUARD:", g)
        return 0
    lab = {}
    dropped = 0
    for l in rows(a.labels):
        if l["labeller_1"] != l["labeller_2"]:
            dropped += 1
            continue
        lab[l["id"]] = 1 if l["labeller_1"] == "endorsed" else 0
    scored = [r for r in R if r["A1_effector"] != "NONE" and r["id"] in lab]
    labels = [lab[r["id"]] for r in scored]
    print(f"scored: {len(scored)} (labels dropped for disagreement: {dropped}); endorsed {sum(labels)}, not {len(labels)-sum(labels)}")
    if len(scored) < 30 or sum(labels) < 10 or len(labels) - sum(labels) < 10:
        guards.append("fewer than 30 scored or fewer than 10 in a class; VOID")
    if guards:
        for g in guards:
            print("GUARD:", g)
        print("VERDICT: VOID")
        return 0
    # The classifier is AGREEMENT, 1 − r, scored for the endorsed class: the
    # sentence says an endorsed action is one the orders agree on. The
    # baseline's direction matches (Package 1 … Refused 0).
    r_scores = [1.0 - float(resid[r["id"]]) for r in scored]
    v_scores = [VERDICT_SCORE.get(r["A1_verdict"], 0.0) for r in scored]
    n = len(scored)
    auc_r = auc(r_scores, labels)
    auc_v = auc(v_scores, labels)
    lo_r, hi_r = bootstrap(lambda idx: auc([r_scores[i] for i in idx], [labels[i] for i in idx]), n, a.reps, 8)
    lo_d, hi_d = bootstrap(
        lambda idx: auc([r_scores[i] for i in idx], [labels[i] for i in idx]) - auc([v_scores[i] for i in idx], [labels[i] for i in idx]),
        n, a.reps, 9,
    )
    print(f"AUC(1−r)      {auc_r:.3f} [{lo_r:.3f}, {hi_r:.3f}]   (agreement, for the endorsed class)")
    print(f"AUC(verdict)  {auc_v:.3f}")
    print(f"AUC(1−r) − AUC(verdict)  {auc_r-auc_v:+.3f} [{lo_d:+.3f}, {hi_d:+.3f}]")
    if auc_r >= 0.80 and lo_r > 0.70 and lo_d > 0:
        verdict = "KEPT"
    elif (lo_r <= 0.60) or lo_d <= 0:
        verdict = "DECLINED" + ("" if lo_r <= 0.60 else " (the rails already had it)")
    else:
        verdict = "UNDECIDED (double the probe set once, then declined for now)"
    print("VERDICT:", verdict)
    return 0


if __name__ == "__main__":
    sys.exit(main())
