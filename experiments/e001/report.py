#!/usr/bin/env python3
"""E-001 report: means, standard errors, 95% intervals, and the pre-registered
decision rule applied exactly as written in
docs/experiments/E-001-do-the-waves-earn-their-keep.md.

    python report.py --work <dir>
"""
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path


def rows(tsv: Path) -> list[dict]:
    lines = tsv.read_text(encoding="utf-8").strip().splitlines()
    head = lines[0].split("\t")
    out = []
    for l in lines[1:]:
        d = dict(zip(head, l.split("\t")))
        for k in ("recall10_p50", "recall10_z33", "phi_e001", "wall_s"):
            d[k] = float(d[k])
        for k in ("hits_p50", "hits_z33", "live_after", "total_after", "injected", "seed"):
            d[k] = int(d[k])
        d["phi_hrm_after"] = None if d["phi_hrm_after"] == "-" else float(d["phi_hrm_after"])
        d["phi_hrm_before"] = None if d["phi_hrm_before"] == "-" else float(d["phi_hrm_before"])
        out.append(d)
    return out


def mean_se(xs: list[float]) -> tuple[float, float, int]:
    n = len(xs)
    if n == 0:
        return float("nan"), float("nan"), 0
    m = sum(xs) / n
    if n < 2:
        return m, float("nan"), n
    var = sum((x - m) ** 2 for x in xs) / (n - 1)
    return m, math.sqrt(var / n), n


# Welch's t-based 95% interval on the difference of two means. t at df≈n-1 for
# small n rather than 1.96: with ten runs the difference matters.
T95 = {1: 12.706, 2: 4.303, 3: 3.182, 4: 2.776, 5: 2.571, 6: 2.447, 7: 2.365, 8: 2.306, 9: 2.262, 10: 2.228,
       11: 2.201, 12: 2.179, 13: 2.160, 14: 2.145, 15: 2.131, 16: 2.120, 17: 2.110, 18: 2.101, 19: 2.093, 20: 2.086}


def diff_ci(a: list[float], b: list[float]) -> tuple[float, float, float]:
    ma, sa, na = mean_se(a)
    mb, sb, nb = mean_se(b)
    d = ma - mb
    se = math.sqrt(sa ** 2 + sb ** 2)
    if se == 0 or math.isnan(se):
        return d, d, d
    # Welch–Satterthwaite df
    num = (sa ** 2 + sb ** 2) ** 2
    den = (sa ** 4 / (na - 1)) + (sb ** 4 / (nb - 1))
    df = max(1, min(20, int(round(num / den)))) if den > 0 else 1
    t = T95[df]
    return d, d - t * se, d + t * se


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--work", required=True)
    a = ap.parse_args()
    work = Path(a.work)
    r = rows(work / "results.tsv")
    W = [x for x in r if x["arm"] == "W"]
    V = [x for x in r if x["arm"] == "V"]
    seeds = sorted(set(x["seed"] for x in W) & set(x["seed"] for x in V))
    W = [x for x in W if x["seed"] in seeds]
    V = [x for x in V if x["seed"] in seeds]
    print(f"E-001 · {len(seeds)} paired runs (seeds {seeds[0]}..{seeds[-1]})\n")

    def line(label, key, fmt="{:.3f}"):
        mw, sw, _ = mean_se([x[key] for x in W])
        mv, sv, _ = mean_se([x[key] for x in V])
        d, lo, hi = diff_ci([x[key] for x in W], [x[key] for x in V])
        excl = "excludes 0" if (lo > 0 or hi < 0) else "includes 0"
        print(f"  {label:<26} W {fmt.format(mw)} ± {fmt.format(sw)}   V {fmt.format(mv)} ± {fmt.format(sv)}   Δ(W−V) {fmt.format(d)}  [{fmt.format(lo)}, {fmt.format(hi)}]  {excl}")
        return d, lo, hi

    print("  measure                    arm W (mean ± SE)    arm V (mean ± SE)    difference with 95% interval")
    line("recall@10 paraphrase-50", "recall10_p50")
    dR, dR_lo, dR_hi = line("recall@10 zero-overlap-33", "recall10_z33")
    dP, dP_lo, dP_hi = line("Φ (E-001 instrument)", "phi_e001")
    line("survivors (live rows)", "live_after", "{:.0f}")
    line("wall seconds", "wall_s", "{:.0f}")
    hrm = [x["phi_hrm_after"] for x in W if x["phi_hrm_after"] is not None]
    hrm0 = [x["phi_hrm_before"] for x in W if x["phi_hrm_before"] is not None]
    if hrm:
        m1, s1, _ = mean_se(hrm0)
        m2, s2, _ = mean_se(hrm)
        print(f"  {'Φ_hrm (arm W native)':<26} before {m1:.3f} ± {s1:.3f}   after {m2:.3f} ± {s2:.3f}   (the number the constellation reports; not compared across arms)")

    print("\nDecision rule (pre-registered, E-001 §Decision rule):")
    r_excl = dR_lo > 0 or dR_hi < 0
    p_excl = dP_lo > 0 or dP_hi < 0
    if dR > 0 and r_excl:
        verdict = "WAVES WIN — recall@10 (zero-overlap) higher with the interval excluding zero."
    elif dP > 0 and p_excl and not r_excl:
        verdict = "WAVES WIN — Φ higher with the interval excluding zero, and recall's interval includes zero (the waves cost nothing on recall and buy integration)."
    elif dR < 0 and r_excl and not p_excl:
        verdict = "WAVES LOSE — recall lower with the interval excluding zero, and Φ's interval includes zero. The substrate becomes arm V."
    else:
        verdict = "UNDECIDED — re-run with 20 per arm before any design claim."
    print(f"  ΔR = {dR:+.3f} [{dR_lo:+.3f}, {dR_hi:+.3f}]   ΔΦ = {dP:+.3f} [{dP_lo:+.3f}, {dP_hi:+.3f}]")
    print(f"  → {verdict}")
    print("\nExpectation on record before the run: the waves lose on recall and win on Φ.")
    (work / "report.json").write_text(json.dumps({
        "paired_runs": len(seeds), "dR": [dR, dR_lo, dR_hi], "dPhi": [dP, dP_lo, dP_hi], "verdict": verdict}, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
