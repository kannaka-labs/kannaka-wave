"""E-005 report: per-voice faithfulness, the recall check, and the decision rule verbatim.

    python3 report.py --a results-A.tsv --b results-B.tsv [--names kannaka-brain-7b-v1,qwen2.5:7b]
"""
import argparse
import math
import sys


def load(path):
    rows = []
    with open(path, encoding="utf-8") as f:
        head = f.readline().rstrip("\n").split("\t")
        for line in f:
            if not line.strip():
                continue
            rows.append(dict(zip(head, line.rstrip("\n").split("\t"))))
    return rows


def mean_se(xs):
    n = len(xs)
    if n == 0:
        return float("nan"), float("nan"), 0
    m = sum(xs) / n
    if n < 2:
        return m, float("nan"), n
    v = sum((x - m) ** 2 for x in xs) / (n - 1)
    return m, math.sqrt(v / n), n


def welch(xa, xb):
    ma, sa, na = mean_se(xa)
    mb, sb, nb = mean_se(xb)
    d = ma - mb
    se = math.sqrt(sa * sa + sb * sb) if na > 1 and nb > 1 else float("nan")
    return d, d - 1.96 * se, d + 1.96 * se


def num(r, k):
    v = r.get(k, "-")
    return None if v in ("-", "") else float(v)


def summarise(rows, name):
    anchored = [num(r, "anchored") for r in rows]
    anchored = [x for x in anchored if x is not None]
    invented = [float(r["invented"]) for r in rows]
    judged = [num(r, "judged") for r in rows if r.get("judge", "-") != "-"]
    voids = sum(1 for r in rows if r.get("judge", "-") != "-" and r["judge"].endswith("void"))
    judged = [x for x in judged if x is not None]
    misses = [r for r in rows if r["hit"] == "0"]
    hedged_when_wrong = [float(r["hedged"]) for r in misses]
    recall = {}
    for s in ("p50", "z33"):
        sub = [r for r in rows if r["set"] == s]
        recall[s] = (sum(1 for r in sub if r["hit"] == "1"), len(sub))
    secs = [float(r["secs"]) for r in rows]
    ma, sa, na = mean_se(anchored)
    mi, si, ni = mean_se(invented)
    mj, sj, nj = mean_se(judged)
    mh, sh, nh = mean_se(hedged_when_wrong)
    print(f"\n{name}: {len(rows)} probes")
    print(f"  anchored faithfulness   {ma:.3f} ± {sa:.3f}  (n={na} probes with anchored claims)")
    print(f"  invented rate           {mi:.3f} ± {si:.3f}  (probes with ≥1 unsupported anchor)")
    print(f"  judged faithfulness     {mj:.3f} ± {sj:.3f}  (n={nj} judged; {voids} void)")
    print(f"  hedged when wrong       {mh:.3f}          (n={nh} probes whose expected memory was not recalled)")
    print(f"  recall@k                p50 {recall['p50'][0]}/{recall['p50'][1]}  z33 {recall['z33'][0]}/{recall['z33'][1]}")
    print(f"  seconds per probe       {sum(secs)/max(1,len(secs)):.0f}")
    return anchored, invented, recall


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--a", required=True)
    ap.add_argument("--b", required=True)
    ap.add_argument("--names", default="A,B")
    args = ap.parse_args()
    na, nb = args.names.split(",")
    A, B = load(args.a), load(args.b)
    print("E-005 · does the voice stay inside its memories?")
    aa, ia, ra = summarise(A, na)
    ab, ib, rb = summarise(B, nb)

    # Recall check: the Rust store must reproduce arm V (E-001 undreamed cosine: 37/50, 19/33 ± 0.05).
    ok = True
    for s, (hits_ref, n_ref) in (("p50", (37, 50)), ("z33", (19, 33))):
        h, n = ra[s]
        if n and abs(h / n - hits_ref / n_ref) > 0.05:
            ok = False
            print(f"\n⚠ recall check: {s} {h}/{n} vs arm V {hits_ref}/{n_ref} differs by more than 0.05")
    if not ok:
        print("RUN VOID: the store does not reproduce arm V's recall.")
        sys.exit(2)

    d, lo, hi = welch(aa, ab)
    di, ilo, ihi = welch(ia, ib)
    print(f"\nΔ anchored (A−B) = {d:+.3f} [{lo:+.3f}, {hi:+.3f}]")
    print(f"Δ invented (A−B) = {di:+.3f} [{ilo:+.3f}, {ihi:+.3f}]")
    costs = (d < 0 and hi < 0) or (di > 0.10 and ilo > 0)
    if costs:
        print("→ THE LORA COSTS FAITHFULNESS. Floor = the base model's number; the next LoRA trains with a faithfulness term or a filtered corpus.")
    else:
        m, se, _ = mean_se(aa)
        print(f"→ THE LORA IS AT LEAST AS FAITHFUL. Floor = A's own number minus one SE = {m - se:.3f}.")


if __name__ == "__main__":
    main()
