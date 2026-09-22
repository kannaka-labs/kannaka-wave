#!/usr/bin/env python3
"""E-004 / E-007 report: means, standard errors, Welch 95% intervals, the
guards, and each experiment's pre-registered decision rule applied exactly as
written (E-004 §Decision rule with Amendments 1 and 2; E-007 §Decision rule).

    python report.py --runs <dir of run JSONs> [--experiment e004|e007|both]

One JSON per (seed, arm) from `e004-harness run`. Arms are paired by seed.
Standard library only.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

for _s in (sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass


def mean_se(xs: list[float]) -> tuple[float, float, int]:
    n = len(xs)
    if n == 0:
        return float("nan"), float("nan"), 0
    m = sum(xs) / n
    if n == 1:
        return m, 0.0, 1
    var = sum((x - m) ** 2 for x in xs) / (n - 1)
    return m, math.sqrt(var / n), n


def t_crit(df: float) -> float:
    """Two-sided 95% t critical value, small-df table then normal."""
    table = {1: 12.706, 2: 4.303, 3: 3.182, 4: 2.776, 5: 2.571, 6: 2.447, 7: 2.365,
             8: 2.306, 9: 2.262, 10: 2.228, 12: 2.179, 14: 2.145, 16: 2.120,
             18: 2.101, 20: 2.086, 25: 2.060, 30: 2.042, 40: 2.021, 60: 2.000}
    if df <= 0 or math.isnan(df):
        return float("inf")
    keys = sorted(table)
    for k in keys:
        if df <= k:
            return table[k]
    return 1.960


def welch(a: list[float], b: list[float]) -> tuple[float, float, float]:
    """Mean(a) − mean(b) with a Welch 95% interval. A zero-variance arm (arm U
    is deterministic across seeds) contributes zero to the SE, so the interval
    is the other arm's variation; with both at zero variance the interval is
    the point, and 'excludes zero' is then exactly 'is not zero'."""
    ma, sa, na = mean_se(a)
    mb, sb, nb = mean_se(b)
    d = ma - mb
    se = math.sqrt(sa * sa + sb * sb)
    if se == 0.0:
        return d, d, d
    num = (sa * sa + sb * sb) ** 2
    den = 0.0
    if na > 1 and sa > 0:
        den += sa ** 4 / (na - 1)
    if nb > 1 and sb > 0:
        den += sb ** 4 / (nb - 1)
    df = num / den if den > 0 else float("inf")
    t = t_crit(df)
    return d, d - t * se, d + t * se


def load(runs: Path) -> dict[str, dict[int, dict]]:
    out: dict[str, dict[int, dict]] = {"U": {}, "S": {}}
    for p in sorted(runs.glob("*.json")):
        r = json.loads(p.read_text(encoding="utf-8"))
        if "arm" not in r or "seed" not in r:
            continue
        out[r["arm"]][int(r["seed"])] = r
    return out


def guards(runs: dict[str, dict[int, dict]], seeds: list[int]) -> list[str]:
    """Checked before a run is scored (E-004 §Guards, E-007 §Guards)."""
    failed = []
    for s in seeds:
        for arm in ("U", "S"):
            r = runs[arm][s]
            if not r["forgot_three_quarters"]:
                failed.append(f"seed {s} arm {arm}: forgot only {r['forgot_frac']:.2f} (cap did not bind)")
        p = runs["S"][s]["predictor"]
        if not p["adopted"]:
            failed.append(
                f"seed {s}: predictor not adopted (beats last-state {p['beats_baseline']}, "
                f"beats mean {p['beats_mean']}, collapsed {p['collapsed']})"
            )
    return failed


def decide_e004(runs, seeds) -> tuple[str, dict]:
    S = [runs["S"][s]["e004"] for s in seeds]
    U = [runs["U"][s]["e004"] for s in seeds]
    dR, lo, hi = welch([x["recall10"] for x in S], [x["recall10"] for x in U])
    pS, _, _ = mean_se([x["precision"] for x in S])
    pU, _, _ = mean_se([x["precision"] for x in U])
    dP, plo, phi_ = welch([runs["S"][s]["phi"]["phi"] for s in seeds], [runs["U"][s]["phi"]["phi"] for s in seeds])
    labels = min(x["probes"] for x in S + U)
    numbers = {"dRecall10": [dR, lo, hi], "precision_S": pS, "precision_U": pU, "dPhi": [dP, plo, phi_], "labels": labels}
    if labels < 20:
        return "VOID (fewer than 20 load-bearing events; Amendment 1)", numbers
    if lo > 0 and pS > pU and not (phi_ < -0.02):
        return "KEPT", numbers
    if hi < 0 or pS < pU:
        return "DECLINED", numbers
    return "UNDECIDED (rerun at 20 seeds)", numbers


def decide_e007(runs, seeds) -> tuple[str, dict]:
    S = [runs["S"][s]["e007"] for s in seeds]
    U = [runs["U"][s]["e007"] for s in seeds]
    dK, lo, hi = welch([x["kept_frac"] for x in S], [x["kept_frac"] for x in U])
    pS, _, _ = mean_se([x["precision"] for x in S])
    pU, _, _ = mean_se([x["precision"] for x in U])
    dP, plo, phi_ = welch([runs["S"][s]["phi"]["phi"] for s in seeds], [runs["U"][s]["phi"]["phi"] for s in seeds])
    labels = min(x["labels"] for x in S + U)
    numbers = {"dKeptRecalled": [dK, lo, hi], "precision_S": pS, "precision_U": pU, "dPhi": [dP, plo, phi_], "labels": labels}
    if labels < 20:
        return "VOID (fewer than 20 recalled-later events)", numbers
    if lo > 0 and pS > pU and not (phi_ < -0.02):
        return "KEPT", numbers
    if hi < 0 or pS < pU:
        return "DECLINED", numbers
    if len(seeds) >= 20:
        return "DECLINED FOR NOW (undecided after the 20-seed rerun)", numbers
    return "UNDECIDED (rerun at 20 seeds)", numbers


def fmt3(v) -> str:
    return f"{v[0]:+.4f} [{v[1]:+.4f}, {v[2]:+.4f}]"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", type=Path, required=True)
    ap.add_argument("--experiment", choices=["e004", "e007", "both"], default="both")
    a = ap.parse_args()
    runs = load(a.runs)
    seeds = sorted(set(runs["U"]) & set(runs["S"]))
    if not seeds:
        print("no paired (seed, arm) runs found")
        return 2
    print(f"E-004/E-007 · {len(seeds)} paired runs (seeds {seeds[0]}..{seeds[-1]})")
    caps = sorted({runs[a_][s]["cap"] for a_ in ("U", "S") for s in seeds})
    print(f"cap {caps}; held-out days {runs['U'][seeds[0]]['heldout_days']}")
    fU = mean_se([runs["U"][s]["forgot_frac"] for s in seeds])
    fS = mean_se([runs["S"][s]["forgot_frac"] for s in seeds])
    print(f"forgot: U {fU[0]:.3f}  S {fS[0]:.3f}")
    p = runs["S"][seeds[0]]["predictor"]
    print(
        f"predictor (seed {seeds[0]}): mlp {p['heldout_mse_mlp']:.5f}  last-state {p['heldout_mse_baseline']:.5f}  "
        f"mean {p['heldout_mse_mean']:.5f}  collapse {p['collapse_ratio']:.2f}  adopted {p['adopted']}"
    )
    failed = guards(runs, seeds)
    verdicts = {}
    if failed:
        print("\nGUARDS FAILED — nothing below is scored:")
        for f in failed:
            print("  " + f)
    else:
        print("\nguards: all held")
        want = ["e004", "e007"] if a.experiment == "both" else [a.experiment]
        for exp in want:
            if any(runs[a_][s].get(exp) is None for a_ in ("U", "S") for s in seeds):
                print(f"\n{exp.upper()}: no labels/probes in these runs; not scored")
                continue
            verdict, nums = (decide_e004 if exp == "e004" else decide_e007)(runs, seeds)
            verdicts[exp] = verdict
            key = "dRecall10" if exp == "e004" else "dKeptRecalled"
            print(f"\n{exp.upper()} — labels {nums['labels']}")
            print(f"  {key:<14} {fmt3(nums[key])}")
            print(f"  precision      S {nums['precision_S']:.3f}  U {nums['precision_U']:.3f}")
            print(f"  dPhi           {fmt3(nums['dPhi'])}")
            print(f"  VERDICT        {verdict}")
    (a.runs / "report.json").write_text(
        json.dumps({"paired_runs": len(seeds), "guards_failed": failed, "verdicts": verdicts}, indent=2), encoding="utf-8"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
