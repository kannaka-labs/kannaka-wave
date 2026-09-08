# E-003: Faithfulness as commutation — the bridge operator at the conscience boundary

**Status:** pre-registered, independent of E-001 (it needs the rails and a fixture set,
not the substrate)
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
