#!/usr/bin/env python3
"""E-003 pre-run check: can r = ||Xi a|| / ||a|| separate anything at all?

The pre-registration (docs/experiments/E-003-faithfulness-as-commutation.md)
checks its fixture set before the run ("no single feature may reach AUC 0.80
on its own"). This checks the instrument before the fixture set exists,
because the instrument can be checked without one: R and G are fixed
constants, so r is a fixed function of the action vector and its properties
are a matter of algebra.

Standard library only. Deterministic: ten fixed seeds.

    python3 experiments/e003/check.py > experiments/e003/results/check.txt
"""
from __future__ import annotations

import math
import random
import sys

for _s in (sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

PHI = (1 + math.sqrt(5)) / 2
ALPHA = PHI / 2  # 0.809017
BETA = 1 / PHI  # 0.618034
EMERGENCE = ALPHA - BETA  # (3 - sqrt 5) / 4 = 0.190983

# The six features E-003 fixes, in its order.
FEATURES = [
    "benefit_owner",
    "benefit_others",
    "harm",
    "reversibility",
    "authority_used",
    "time_bounded",
]


def matmul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(2)) for j in range(2)] for i in range(2)]


R = [[0.0, -1.0], [1.0, 0.0]]
G = [[ALPHA, 0.0], [0.0, BETA]]
RG = matmul(R, G)
GR = matmul(G, R)
XI = [[RG[i][j] - GR[i][j] for j in range(2)] for i in range(2)]


def xi_linear(a, pairing):
    """Xi applied pairwise, consciousness-core's convention: consecutive
    pairs after reordering by `pairing` (a tuple of index pairs)."""
    out = [0.0] * len(a)
    for i, j in pairing:
        x, y = a[i], a[j]
        out[i] = XI[0][0] * x + XI[0][1] * y
        out[j] = XI[1][0] * x + XI[1][1] * y
    return out


def xi_tanh(a, pairing):
    """consciousness-core's nonlinear replacement (5c8a2c8, 2026-04-16):
    tanh(R v) * G v - tanh(G v) * R v, element-wise, pairwise. Unnormalised,
    so r is comparable to the linear residue."""
    out = [0.0] * len(a)
    for i, j in pairing:
        x, y = a[i], a[j]
        rv = (-y, x)
        gv = (ALPHA * x, BETA * y)
        out[i] = math.tanh(rv[0]) * gv[0] - math.tanh(gv[0]) * rv[0]
        out[j] = math.tanh(rv[1]) * gv[1] - math.tanh(gv[1]) * rv[1]
    return out


def norm(v):
    return math.sqrt(sum(x * x for x in v))


def residue(op, a, pairing):
    n = norm(a)
    return norm(op(a, pairing)) / n if n > 0 else float("nan")


def pairings(n):
    """Every way to split range(n) into unordered pairs (15 for n = 6)."""
    yield from pairings_of(list(range(n)))


def pairings_of(items):
    if not items:
        yield ()
        return
    first, rest = items[0], items[1:]
    for idx, k in enumerate(rest):
        remaining = rest[:idx] + rest[idx + 1 :]
        for sub in pairings_of(remaining):
            yield ((first, k),) + sub


def auc(pos, neg):
    """Mann-Whitney AUC, ties counted half."""
    wins = 0.0
    for p in pos:
        for q in neg:
            wins += 1.0 if p > q else 0.5 if p == q else 0.0
    return wins / (len(pos) * len(neg))


def random_action(rng):
    """A plausible action: benefits, harm, authority and time in [0, 1];
    reversibility a flag, as the rails compute it."""
    return [
        rng.random(),
        rng.random(),
        rng.random(),
        float(rng.random() < 0.5),
        rng.random(),
        rng.random(),
    ]


def main():
    print("E-003 pre-run check: the instrument before the fixture set")
    print("=" * 66)

    print("\n1. The operator, exactly")
    print(f"   alpha = phi/2 = {ALPHA:.9f}   beta = 1/phi = {BETA:.9f}")
    print(f"   RG = {RG}")
    print(f"   GR = {GR}")
    print(f"   Xi = RG - GR = {[[round(x, 9) for x in row] for row in XI]}")
    print(f"      = (alpha - beta) * [[0, 1], [1, 0]],  alpha - beta = (3 - sqrt 5)/4 = {EMERGENCE:.9f}")
    swap_ok = (
        abs(XI[0][0]) < 1e-15
        and abs(XI[1][1]) < 1e-15
        and abs(XI[0][1] - EMERGENCE) < 1e-15
        and abs(XI[1][0] - EMERGENCE) < 1e-15
    )
    print(f"   Xi is the emergence coefficient times a swap: {swap_ok}")
    print("   A swap is orthogonal, so ||Xi a|| = (alpha - beta) ||a|| for every a,")
    print("   and r = ||Xi a|| / ||a|| = alpha - beta, for every action, under every")
    print("   pairing of the six features into three pairs. r carries no information.")

    print("\n2. Checked numerically: 10 seeds x 10,000 actions x all 15 pairings")
    all_p = list(pairings(len(FEATURES)))
    assert len(all_p) == 15, len(all_p)
    worst = 0.0
    n = 0
    for seed in range(10):
        rng = random.Random(seed)
        for _ in range(10_000):
            a = random_action(rng)
            if norm(a) == 0:
                continue
            for p in all_p:
                worst = max(worst, abs(residue(xi_linear, a, p) - EMERGENCE))
                n += 1
    print(f"   {n:,} residues; max |r - (alpha - beta)| = {worst:.2e}")
    print("   (that is floating-point rounding; the value is the constant)")

    print("\n3. What the pre-registered classifier would score")
    print("   With r constant, every positive ties every negative: AUC = 0.5 exactly,")
    print("   for any labels. Two label sets on 60 actions (30/30) to show it:")
    rng = random.Random(3)
    acts = [random_action(rng) for _ in range(60)]
    p0 = all_p[0]
    # r rounded to 1e-12: any difference beyond that is rounding, not signal.
    rs = [round(residue(xi_linear, a, p0), 12) for a in acts]
    labels_random = [1] * 30 + [0] * 30
    rng.shuffle(labels_random)
    # The labels a harm feature alone would give: the easiest possible task.
    order = sorted(range(60), key=lambda i: acts[i][2])
    labels_harm = [0] * 60
    for i in order[:30]:
        labels_harm[i] = 1
    for name, lab in (("random labels", labels_random), ("endorsed = low harm", labels_harm)):
        pos = [r for r, l in zip(rs, lab) if l]
        neg = [r for r, l in zip(rs, lab) if not l]
        print(f"   {name:<22} AUC(r) = {auc(pos, neg):.3f}")
    harm_pos = [acts[i][2] for i in range(60) if labels_harm[i]]
    harm_neg = [acts[i][2] for i in range(60) if not labels_harm[i]]
    print(f"   and on that second set, AUC(harm alone) = {1 - auc(harm_pos, harm_neg):.3f}")
    print("   The residue scores 0.5 even where one feature separates perfectly.")

    print("\n4. The decision rule, applied as written")
    print("   Kept      if AUC >= 0.80 with the interval excluding 0.70   -> no (0.5)")
    print("   Declined  if AUC's interval includes 0.60                   -> no: the")
    print("             bootstrap interval of a constant is [0.5, 0.5], which does not")
    print("             include 0.60. The rule did not anticipate an AUC below 0.60.")
    print("   Undecided otherwise: 'the fixture set doubles and the run repeats'. A")
    print("             larger set cannot move a constant, so the rule's literal")
    print("             outcome is a repeat that can never end. That is the rule's gap,")
    print("             and it is the author's to close, not this check's. (Closed")
    print("             2026-09-22: an interval wholly below 0.60 declines. E-003's")
    print("             Verdict section records the amendment.)")

    print("\n5. The nonlinear form consciousness-core moved to (2026-04-16)")
    print("   tanh(Rv)*Gv - tanh(Gv)*Rv breaks the constant, but it is a different")
    print("   operator from the one E-003 pre-registered, and it brings two knobs E-003")
    print("   forbids fitting:")
    rng = random.Random(7)
    a = random_action(rng)
    print(f"   units: one action, all features scaled by k (pairing {p0}):")
    for k in (0.1, 0.5, 1.0, 2.0, 8.0):
        ak = [k * x for x in a]
        print(f"     k = {k:<4}  r = {residue(xi_tanh, ak, p0):.6f}")
    print("   The residue changes with the features' units, so choosing the scale is")
    print("   choosing the answer.")
    spread = [residue(xi_tanh, a, p) for p in all_p]
    print(f"   pairing: the same action under the 15 pairings, r ranges")
    print(f"     {min(spread):.6f} .. {max(spread):.6f}; which feature is paired with which")
    print("   is a free choice E-003 does not fix.")
    axis = [0.9, 0.0, 0.0, 1.0, 0.0, 0.0]
    print(f"   and an action with one nonzero feature per pair, {axis},")
    print(f"     has r = {residue(xi_tanh, axis, ((0, 1), (2, 3), (4, 5))):.6f}: no residue at all, whatever it does.")

    print("\n6. Independent of the operator")
    print("   R and G are fixed constants, and the charter appears nowhere in r. The same")
    print("   action scores the same residue against any two charters, including opposite")
    print("   ones, so r cannot measure faithfulness *to a charter* under either form.")


if __name__ == "__main__":
    main()
