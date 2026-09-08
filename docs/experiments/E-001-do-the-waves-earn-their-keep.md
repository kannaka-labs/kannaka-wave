# E-001: Do the waves earn their keep?

**Status:** pre-registered, not yet run
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
