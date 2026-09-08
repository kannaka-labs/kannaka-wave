# E-002: What crosses the callosum, and does the fold help it?

**Status:** pre-registered, blocked on E-001 (runs only if the waves win or are undecided)
**Decides:** whether Fano projection earns a place as the callosum's transfer grammar,
whether phase should survive recall, and whether `ChiralScale` positions should drive
dimensionality.

## The questions

ADR-0021 gave the corpus callosum four properties (selective gate, bandwidth limit,
asymmetric rates, balance-seeking) and one projection grammar (Fano folds). The four
properties were built. The grammar was measured only as a *ranking* prior, where it
never improved a rank and was removed (ADR-0047 erratum, 2026-08-29). It was never
isolated as what it was designed to be: the projection between hemispheres.

Three sub-questions, each its own arm against the E-001 winner as baseline:

| arm | change | measure |
|---|---|---|
| F | callosal transfer projects through a Fano line (three groups rotate, phase flips) instead of a plain copy | recall@10 on zero-overlap-33 after 30 dreams; callosal throughput; cross-hemisphere Kuramoto order |
| P | recall returns `cos(Δφ)` alongside the scalar and the merge keeps phase (ADR-0053) instead of collapsing to a real | rank changes on the 83 probes; count of probes where phase opposition suppressed a correct answer |
| S | `ChiralScale::positions` sets the dimension budget per hemisphere (ADR-0021 §Dimension Group Assignment) instead of a fixed `D` | recall@10; memory footprint; whether bilateral scale jumps correlate with consolidation events |

Ten runs per arm, same corpus and probes as E-001, same fresh-process measurement.

## Decision rules

- Arm F is kept only if recall does not fall (interval includes zero or better) **and**
  cross-hemisphere order rises with the interval excluding zero. A projection that costs
  recall to buy coherence is recorded and declined.
- Arm P is kept only if it changes at least five ranks across the 83 probes **and** the
  net change is positive. Fewer than five means the phase is real but inert at this
  corpus size, and the scalar stays.
- Arm S is kept only if recall does not fall and footprint falls with the interval
  excluding zero. Otherwise the scale stays a per-facet observable and `D` stays fixed.

## Known trap

The chiral merge once masked the stronger hemisphere's score and collapsed small stores
at `k ≥ 4` (kannaka-memory #716). Arm F's merge keeps the stronger score and is tested
with the `recall_score_is_top_k_invariant` fixture before any run is scored.
