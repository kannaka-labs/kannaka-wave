# E-001: Do the waves earn their keep?

**Status:** RUN 2026-09-08/09, ten seeds per arm. **Verdict: WAVES LOSE.** See §Result.
**Decides:** whether Kannaka Wave's substrate is a wave medium or a vector store with a
forgetting policy. Nothing in the substrate beyond the encoder, the facets and the
triage is written until this has an answer.

## The question

On the same embeddings, the production medium scored 0.62 recall@10 where plain cosine
scored 0.80 (2026-08-02, frozen 615-memory snapshot). The medium costs 0.18 of recall.
The record says the waves might buy something else: forgetting that raises integration
(Φ 0.26 → 0.50 on the witness store), and coupling that stabilises beliefs into
recall-reliable structures (L7, prediction 1 at K = 0.2). This experiment asks whether
either purchase is real when everything else is held equal.

## Two arms, everything else identical

| | arm W (wave) | arm V (vector) |
|---|---|---|
| encoder | mxbai-embed-large, 1024d | same model, same vectors |
| write | facet decomposition, resolve-only parent | same facets, same parents |
| store | chiral medium: two hemispheres, callosum, energy, phase, `Ξ` coupling | flat matrix of unit vectors + per-row metadata |
| recall | resonance: `similarity × effective_strength × cos(Δφ)`, chiral merge keeping the stronger score | cosine top-k, confirmation-weighted by the same temporal rule |
| dream | annealing + `Ξ` cross-callosal step + tiered triage | tiered triage only, same policy table |
| forgetting policy | identical `[retention]` table, per content class | identical |

The wave arm carries the constellation's confirmed optimum from 2026-06-12 and 2026-08-01
(K = 2.0 transfer / 1.0 eval, chiral bias 0.15, relax 16/20, repulsion 0.28, gravity
0.35) and does not sweep them. The knob search is closed; this is not it.

## Corpus and probes

- Corpus: the frozen 615-memory snapshot, sha `339e7ad9…`, UUID ground truth preserved.
  Both arms absorb the same facet stream in the same order.
- Probes: paraphrase-50 and zero-overlap-33 from the Harbor evals. The zero-overlap
  invariant (no ≥4-char token shared with the target's full content) is re-verified at
  score time; a broken invariant is an infrastructure error and the run is not scored.
- Dream schedule: 30 cycles per run, strong→weak alternation per the L7 finding, with
  the same distractor churn injected into both arms.

## Measurements

Per run, per arm, from a fresh process (never from the dream's own report):

| measure | how |
|---|---|
| recall@10 | both probe sets, after absorb and after 30 dreams |
| Φ | `observe --json` before the first dream and after the 30th |
| memory count | after each dream, the trajectory |
| belief cores | count and lifetime across dreams (arm W only; arm V has no cores by construction, and that is recorded, not scored) |
| wall time | absorb, one recall, one dream |

Ten runs per arm. Report mean and standard error; `SE = σ/√n` is computed before any
comparison is stated.

## Decision rule, fixed now

Let ΔR = recall@10(W) − recall@10(V) after dreams, on the harder zero-overlap set, and
ΔΦ = Φ(W) − Φ(V) after dreams, both with 95% intervals from the ten runs.

- **Waves win** if ΔR > 0 with the interval excluding zero, **or** ΔΦ > 0 with the
  interval excluding zero **and** ΔR's interval includes zero (the waves cost nothing on
  recall and buy integration).
- **Waves lose** if ΔR < 0 with the interval excluding zero and ΔΦ's interval includes
  zero. The substrate becomes arm V. The chiral scale, the callosum and `Ξ` move to a
  `docs/lineage/` record and out of the build.
- **Undecided** otherwise. The experiment is re-run with 20 per arm before any design
  claim is made either way.

The expectation on record, written before the run so it can be wrong: the waves lose on
recall and win on Φ.

## What would make this experiment lie

- A detector that cannot see: the dissolution counter was structurally zero for a month
  because the energy floor sat above the prune threshold. Both arms' triage is verified
  to delete at least one memory in a fixture run before the real runs start.
- Scores under two names: arm W reports `resonance`, unbounded; the comparison uses the
  rank, never the value.
- Reading the dream's own Φ: never. Fresh process, `observe --json`.
- A corpus that differs between arms by absorb order: the facet stream is written once
  to disk and replayed.

## Result (2026-09-09)

Ten paired seeds, thirty cycles each, churn 40 against a cap of 60, both arms on the
same encoder (`mxbai-embed-large`), the same 1,330 corpus facets and 404 pool facets,
the same retention rule. Raw rows in
[`../../experiments/e001/results/results.tsv`](../../experiments/e001/results/results.tsv),
the report verbatim in
[`../../experiments/e001/results/report.txt`](../../experiments/e001/results/report.txt).

| measure | arm W (mean ± SE) | arm V (mean ± SE) | Δ(W−V) with 95% interval |
|---|---|---|---|
| recall@10 paraphrase-50 | 0.240 ± 0.000 | 0.738 ± 0.002 | −0.498 [−0.503, −0.493], excludes 0 |
| recall@10 zero-overlap-33 | 0.242 ± 0.000 | 0.576 ± 0.000 | −0.333 [−0.333, −0.333], excludes 0 |
| Φ, the shared E-001 instrument | 0.030 ± 0.000 | 0.031 ± 0.000 | −0.001 [−0.001, 0.000], includes 0 |
| survivors after the last dream | 2,306 | 2,306 | 0 |
| wall seconds per seed | 8,161 ± 198 | 23 ± 1 | |
| Φ_hrm, arm W's native reading | before 0.135, after 0.144 ± 0.001 | — | not compared |

**Decision rule applied verbatim:** ΔR = −0.333 with the interval excluding zero and
ΔΦ's interval includes zero. **The waves lose. The substrate becomes arm V.** The
chiral scale moves to [`../lineage/`](../lineage/README.md) and out of the build; the
callosum and `Ξ` never reached code here and stay in the record. E-002 is declined.
E-003 is unaffected.

The expectation on record was that the waves lose on recall and win on Φ. The first
half held; the second did not. Φ on the shared instrument was identical between arms
to three decimals. Arm W's *native* Φ rose from 0.135 to 0.144, which is the number
the constellation reports about itself; the instrument that reads both arms the same
way saw nothing. The spiral log explains the gap: with facets the medium sits at
Kuramoto order 0.98 with zero cores and zero winding, i.e. the whole field
phase-locked. Native Φ rises with that lock; integration over the survivors' actual
similarity structure does not.

**What varied and what did not.** Arm W's recall was identical across all ten seeds
(12 of 50, 8 of 33). The seed enters only through the dream, and the dream's effect on
recall is deterministic at this size, so every wave seed was a replicate. Arm V varied
by one hit on one seed. The intervals are as tight as they look because the arms are
that stable, not because n is large.

**The attribution pair, not part of the decision.** The same run without facets in
either arm (`E001_NO_FACETS=1`, three seeds): W0 0.280 / 0.212, V0 0.760–0.780 /
0.606–0.636, Φ equal. Build-only, the no-facet medium had scored 24 of 50 and 15 of
33 (harness README, finding 5). After thirty wave dreams it held 14 and 7. **Dreaming
cost the medium recall**; arm V's dream is only forgetting and its recall did not move.
The facet-bearing medium started lower (15 and 12) and ended at 12 and 8. So the gap
has two parts of roughly equal size: facets interfere on the write side, and the wave
dynamics erode what is left.

**Deviations from the spec, all recorded before the run** (harness README, findings
1–5): arm W dreams wave-native (phases 1, 3, 4; the legacy particle consolidation does
not finish one dream at this size), triage is applied through `triage_forget` because
the stage's own ghosts revert within the dream, and recall runs on production knobs
because the eval knob is unsafe on a dreamed store. None of these favour arm V: the
first removes a phase that ADR-0022 diagnosed as harmful to the waves, the second is
applied identically to both arms, and the third is the substrate's own default.

**Provenance.** kannaka-memory `441f6edc`, kannaka-wave harness `75819f63`,
`corpus-slim.json` sha256 `57c6df3f…` (the slim export of the frozen snapshot).
debain2, 20 cores, one harness process per seed. The box rebooted mid-run at 08:21 on
2026-09-09 (a fleet redeploy from another session); seeds 1, 5, 8 and the attribution
pair had completed, seeds 2, 6, 9 were restarted from a fresh build, and 3, 4, 7, 10
had not started. A wave seed is deterministic, so the restart changed nothing that the
table would show.

