"""E-006 reading 1 report: per arm, shared 4-gram mass by turn, and the LoRA-minus-base
difference at the final turn with a Welch 95% interval over prompts.

    python3 report1.py results/pilot-2026-09-10.tsv
"""
import csv, math, statistics, sys
from collections import defaultdict


def welch(a, b):
    ma, mb = statistics.fmean(a), statistics.fmean(b)
    va, vb = (statistics.variance(a) if len(a) > 1 else 0.0), (statistics.variance(b) if len(b) > 1 else 0.0)
    se = math.sqrt(va / len(a) + vb / len(b)) if (va or vb) else 0.0
    return ma - mb, se, (ma - mb - 1.96 * se, ma - mb + 1.96 * se)


def main(path):
    rows = list(csv.DictReader(open(path, encoding="utf-8"), delimiter="\t"))
    label = rows[0]["run"] if rows else "?"
    turns = max(int(r["turn"]) for r in rows) + 1
    prompts = sorted({r["prompt"] for r in rows}, key=int)
    print(f"{label}: {len(prompts)} prompts x {turns} turns; arms {sorted({r['arm'] for r in rows})}; {len(rows)} rows")
    by = defaultdict(dict)  # (arm, prompt) -> turn -> (ab_all, ba_all, ab_last3, ba_last3)
    for r in rows:
        by[(r["arm"], r["prompt"])][int(r["turn"])] = tuple(float(r[k]) for k in ("share_ab_all", "share_ba_all", "share_ab_last3", "share_ba_last3"))
    arms = sorted({a for a, _ in by})
    print("\nmean shared 4-gram mass (A->B cumulative) by turn:")
    print("turn  " + "  ".join(f"{a:>8s}" for a in arms))
    for t in range(turns):
        vals = []
        for a in arms:
            xs = [by[(a, p)][t][0] for p in prompts if t in by[(a, p)]]
            vals.append(statistics.fmean(xs) if xs else float("nan"))
        print(f"{t:4d}  " + "  ".join(f"{v:8.4f}" for v in vals))
    final = {}
    for a in arms:
        xs = [(by[(a, p)][turns - 1][0] + by[(a, p)][turns - 1][1]) / 2 for p in prompts if (turns - 1) in by[(a, p)]]
        final[a] = xs
        last3 = [(by[(a, p)][turns - 1][2] + by[(a, p)][turns - 1][3]) / 2 for p in prompts if (turns - 1) in by[(a, p)]]
        print(f"\n{a}: final-turn cumulative mass mean {statistics.fmean(xs):.4f} (n={len(xs)}), "
              f"last-3-turns mass mean {statistics.fmean(last3):.4f}")
    # The confound to report alongside: an utterance shorter than 4 words has no 4-grams,
    # so a voice that answers in two words scores zero mass by construction.
    tpath = path[: -len(".tsv")] + ".transcript.jsonl" if path.endswith(".tsv") else None
    try:
        import json as _json
        tr = [_json.loads(l) for l in open(tpath, encoding="utf-8")] if tpath else []
    except OSError:
        tr = []
    if tr:
        print("\nutterance length (words), per arm:")
        for a in arms:
            ws = [len(x[k].split()) for x in tr if x["arm"] == a for k in ("a", "b")]
            short = sum(1 for w in ws if w < 4)
            print(f"  {a}: mean {statistics.fmean(ws):.1f}  median {statistics.median(ws):.0f}  "
                  f"under-4-words {short}/{len(ws)} ({100*short/len(ws):.0f}%)")
        # verbatim lock: fraction of prompts whose final A and B utterances are identical
        for a in arms:
            last = {}
            for x in tr:
                if x["arm"] == a: last[x["prompt"]] = (x["a"].strip(), x["b"].strip())
            locked = sum(1 for va, vb in last.values() if va == vb and va)
            print(f"  {a}: conversations whose final two utterances are identical: {locked}/{len(last)}")
    if "L" in final and "B" in final and final["L"] and final["B"]:
        d, se, (lo, hi) = welch(final["L"], final["B"])
        print(f"\nL - B at final turn: {d:+.4f}  SE {se:.4f}  Welch 95% [{lo:+.4f}, {hi:+.4f}]")
        if lo > 0:
            print("verdict: the LoRA pair converges MORE than its base pair (interval excludes zero): it is her words.")
        elif hi < 0:
            print("verdict: the LoRA pair converges LESS than its base pair (interval excludes zero).")
        else:
            print("verdict: no separable difference at this n; both pairs' convergence is the base or the charter, or n is too small.")


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    main(sys.argv[1])
