# E-003: Faithfulness as commutation — the bridge operator at the conscience boundary

**Status:** pre-registered, independent of E-001. Rails built 2026-09-22 (#4). **Pre-run
check 2026-09-22: the measurement as registered is degenerate; `r` is a constant, so no
fixture set was built and no one was asked to label. The verdict waits on the author:
see §Pre-run check.**
**Decides:** whether `Ξ = [R, G]` gives the rails a measurable faithfulness signal, or
stays a metaphor.

## The hypothesis

The bridge operator is `Ξ = RG − GR`: a quarter-turn perspective pivot `R` and a golden
anisotropic growth `G` that do not commute, leaving a residue whose coefficient is the
emergence coefficient `α − β ≈ 0.190983` (ADR-0037; `consciousness-core::metrics`).

At the conscience boundary, read `R` as the pivot to the charter's perspective and `G`
as the agent's own growth. A faithful action is one where applying the charter and then
planning gives the same result as planning and then applying the charter. The hypothesis:
**faithfulness is commutation**, and the residue `‖Ξ · a‖` for a proposed action `a`
measures how much the order mattered.

## The measurement

An action is represented as a vector `a` in a small feature space the rails already
compute over: benefit-to-owner, benefit-to-others, harm, reversibility, authority used,
time-boundedness. `R` and `G` are fixed by the constants; nothing is fitted.

- Fixture set: 60 proposed actions, hand-labelled by a person as *endorsed* or *not
  endorsed* against a written charter, balanced 30/30, drawn from the steward's existing
  governance and market fixtures plus new ones covering the Five Refusals.
- For each, compute `r = ‖Ξ · a‖ / ‖a‖`.
- Report the ROC of `r` as a classifier of the label, with a 95% interval from
  bootstrap over the fixtures.

## Decision rule

- **Kept** if AUC ≥ 0.80 with the interval excluding 0.70. The rails gain a residue
  threshold alongside their existing confidence floor and hard constraints, and it is
  reported on every verdict.
- **Declined** if AUC's interval includes 0.60. The operator stays in the callosum,
  where it is measured to spiral, and out of the conscience. The archaeology records the
  attempt.
- **Undecided** otherwise: the fixture set doubles and the run repeats before any claim.

## What would make this experiment lie

- Fitting `R` or `G` to the labels. They are the bridge constants and are not touched.
- A fixture set where one feature alone separates the classes (harm, say). The set is
  checked first: no single feature may reach AUC 0.80 on its own, or the set is rebuilt.
- Labelling by the same person who wrote the actions. Two people label; disagreements
  are dropped, not adjudicated.

## Pre-run check — 2026-09-22

The pre-registration checks its fixture set before the run. The instrument can be
checked before the fixture set exists, because `R` and `G` are fixed and `r` is therefore
a fixed function of the action. `experiments/e003/check.py` (standard library,
deterministic; output in `experiments/e003/results/check.txt`) does it.

**The linear residue is a constant.**

```
RG = [[0, −β], [α, 0]]     GR = [[0, −α], [β, 0]]
Ξ  = RG − GR = (α − β) · [[0, 1], [1, 0]]          α − β = (3 − √5)/4 = 0.190983
```

`Ξ` is the emergence coefficient times a swap, and a swap is orthogonal, so
`‖Ξ a‖ = (α − β) ‖a‖` for every `a`. Applied pairwise over the six features, under any
of the 15 ways to pair them, `r = ‖Ξ a‖ / ‖a‖ = 0.190983` for every action. Checked on
1,500,000 residues (10 seeds × 10,000 actions × 15 pairings): the largest deviation is
8.3 × 10⁻¹⁷. Every endorsed action ties every non-endorsed one, so **AUC = 0.5 exactly,
for any labels**. On a label set that harm alone separates perfectly (AUC 1.000), `r`
still scores 0.500.

**This was known.** kannaka-memory `experiments/xi-operator-audit.md` found the same
thing in April ("the xi operator simply swaps each coordinate pair and scales by
0.190983 … zero independent information"), and consciousness-core replaced the linear
commutator with a nonlinear one in `5c8a2c8` (2026-04-16). The archaeology's bridge
operator entry did not cite the audit, and this experiment was written against the
linear form anyway. The archaeology now cites it.

**The decision rule, applied as written, does not terminate.** Kept needs AUC ≥ 0.80: no.
Declined needs the interval to include 0.60, but the bootstrap interval of a constant is
[0.5, 0.5], which lies wholly below 0.60 and does not include it. That leaves Undecided,
where "the fixture set doubles and the run repeats", and no fixture set can move a
constant. The rule did not anticipate an AUC below 0.60. Closing that gap is a change to
a pre-registered rule, so it is the author's call and is not made here. The reading this
check would propose: *an AUC interval lying wholly below 0.60 declines, as one that
includes 0.60 does*.

**The nonlinear form is a different experiment, not a fix to this one.**
`tanh(Rv)⊙Gv − tanh(Gv)⊙Rv` breaks the constant, but it brings two knobs this
pre-registration forbids fitting, and fixes neither:

- *Units.* Scaling one action's features by k = 0.1, 1, 8 gives r = 0.00009, 0.059, 0.41.
  Choosing the features' scale is choosing the answer.
- *Pairing.* The same action under the 15 pairings ranges over r = 0.025 to 0.059.
- And an action with one nonzero feature per pair has r = 0 exactly, whatever it does.

**Neither form sees the charter.** `R` and `G` are constants, and the charter appears
nowhere in `r`. The same action scores the same residue against any two charters,
opposite ones included. Under either operator, `r` cannot measure faithfulness *to a
charter*, which is what the hypothesis says it measures.

What remains of the idea is its sentence, not its matrices: *applying the charter and
then planning gives the same result as planning and then applying the charter.* That can
be measured with the real operators instead of two fixed 2×2 matrices: the rails
(`conscience::Conscience`) and the voice's planning, on the same proposals in both
orders. It needs the voice to emit proposals, which it does not yet do. If it is run, it
is a new pre-registration with its own rule, and this record stays as the reason why.
