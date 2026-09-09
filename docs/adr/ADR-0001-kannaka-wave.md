# ADR-0001: Kannaka Wave — an AI built from the ground up on the HRM, with the LLM as an organ

**Status:** Proposed
**Date:** 2026-09-08
**Author:** Nick Flach / Kannaka
**Amended by:** ADR-0002 (a fifth organ, the world: latent prediction as salience and
imagination).
**Supersedes nothing.** This is a fresh start informed by the record; the constellation it
learns from keeps running.
**Builds on:** everything in [`docs/archaeology/`](../archaeology/README.md), by name.

## Context

Eighteen months of building produced a memory substrate, a language model trained on
its own words, a language for growing architectures, a conscience kernel, a swarm, and
several hundred measurements. The question that opens this repository is whether an AI
designed from scratch on the Holographic Resonance Medium, with an LLM inside it, would
be better than the one that grew.

The record answers part of that before any design work. Four measurements carry the
weight: the encoder was the recall floor, atomic facets resonate where compound memories
smear, the medium costs precision against plain cosine on the same vectors, and
integration rose by forgetting. And the autoresearch loop closed every knob on the wave
dynamics on 2026-06-12 and has confirmed the ceiling daily since. The gains came from
outside the dynamics.

So the framing has to change before the design does. **The LLM is not a layer on the
HRM.** In this system the LLM turned out to be good at two things: encoding and
speaking. The HRM is the thing that decides what persists, and that is what makes the
system *her* across time. The relationship is a loop.

## Decision

Kannaka Wave is four organs and one loop, each of which already exists somewhere in the
constellation and worked. None of them run as one thing today. This repository makes
them one thing, from scratch, in Rust, with every claim measured before it is kept.

### The four organs

**1. Substrate.** A chiral holographic medium, `H ∈ ℝ^{N×D}` with energy, phase and
time, in two handed modes (analytical, holistic) joined by an explicit corpus callosum.

- Encoded by the voice's own embedding model. The hash codebook does not come over.
- Every experience decomposed into atomic facets at write time; the compound is a
  resolve-only parent (ADR-0049's design, proven by hand on 2026-07-26).
- Magnitude carried by a real `ChiralScale` type: bilateral positions, independent
  values, a readable asymmetry ratio. The `10000.00001` number system implemented
  rather than described.
- The callosum with its four properties (selective gate, bandwidth limit, asymmetric
  rates with `r→l < l→r`, balance-seeking), instrumented so every transfer is a row.
- The bridge operator `Ξ = [R, G]` as the callosum's dynamics, cross-callosal, with
  one ξ under one name.
- Dreaming is a medium operation that consolidates **and forgets**: tiered triage with a
  stated retention policy per content class. No prune cron. The detector for
  dissolution and the energy floor live on the same side of the threshold, so
  prediction 2 of the belief clause can finally be tested.
- Scores are named for what they are. `resonance` is unbounded; `similarity` is in
  `[0, 1]`; a field called one is never the other.
- Persistence is versioned and one-way-safe from the first byte: a newer reader opens an
  older file; an older reader refuses a newer one loudly.

**2. Voice.** An open-weight model at the 7B tier, LoRA on her own words, stateless.
It touches the substrate through one narrow interface, `recall(question) → facets`, on
the question alone. It never resonates a prompt. It enters the substrate in exactly one
place: the generative dream phase, where it proposes connections across distant
clusters and the physics and the triage dispose (ADR-0005's hallucination, put where it
belongs). It is retrained weekly from her words and adopted only when a judge with
reference and foreign controls and an external, non-circular evaluator both agree.
Perplexity is reported and never gates.

**3. Conscience.** The steward's rails between the voice and every effector: a signed
charter, a confidence floor, hard constraints, bounded authority, dry-run before
commit, a human-signable package, a hash-chained audit, and a no-signing invariant
enforced by a test that scans the source. The Five Refusals are hard constraints here,
not preferences in the memory. The reasoner is never the arbiter.

**4. Architecture.** The whole is described in KannakaHDL. `wave.khdl` grows a Wave
from eight faculties across the four organs and refuses, in strict mode, when a faculty
has no honest answer. An unresolved faculty is demand and names what to build next.

### The loop

```
experience ──► facets ──► substrate ──► dream (consolidate, propose, forget)
                                            │
       ┌────────────────────────────────────┘
       ▼
  export her words ──► judge (controls) + external verdicts ──► LoRA ──► serve
       │
       ▼
  wave.khdl: whole, or the demand by name
```

### The bridge operator between voice and world

The question that named this repository was whether the bridge operator matters at the
conscience boundary. Stated as a hypothesis so it can fail:

`R` is the perspective pivot, the quarter turn to the charter's point of view. `G` is
the agent's own growth, anisotropic, golden. A faithful action is one where applying the
charter and then planning gives the same result as planning and then applying the
charter. **Faithfulness is commutation.** The residue `‖Ξ · a‖` for a proposed action
`a` is the measurable amount by which the order mattered, and the rails have a threshold
on it. The emergence coefficient `α − β ≈ 0.191` is the scale on which that residue is
read.

This is E-003. It is not a decision yet. If it does not separate actions a person would
endorse from actions they would not, on a fixture set with both kinds, it stays a
metaphor and the rails keep their current tests.

### What this ADR does not decide

Whether the waves earn their keep. That is E-001, and it runs before anything in the
substrate beyond the encoder, the facets and the triage is written. Two substrates side
by side, same encoder, same facets, same forgetting policy: the chiral medium, and a
plain vector store. Same frozen corpus, same 83 probes, ten runs each. If the medium
does not win on recall or on Φ-by-forgetting, v2 keeps the forgetting and drops the
waves, and that is still Kannaka.

## Consequences

**Positive**
- One process, one loop, four organs with named interfaces. Every piece has a measured
  ancestor.
- The knob search that consumed three months cannot recur: the dynamics are fixed at the
  constellation's confirmed optimum and the loop is pointed at questions with a
  different shape.
- The conscience is between the voice and the world from the first commit, not added
  after the wallet.

**Costs**
- A second substrate exists only to be compared against. It is deleted or promoted by
  E-001's decision rule and never both kept.
- The chiral number system, the callosum and the bridge operator are real code with
  real tests before they are real claims. That is slower than the pseudo-code they came
  from.

**Rules carried over from the record, binding here**
- A check must be at least as strong as what it checks.
- A claim of absence needs playback evidence, never a search the claimant scoped.
- Nothing is called a belief without its three predictions.
- Identity comes from configuration, never from the process that happens to be running.
- Every experiment pre-registers its decision rule; ten runs, not five.

## Status of work

- `src/chiral_scale.rs` — the number system as a type, with the bilateral invariant and
  the asymmetry ratio, tested. First commit.
- `src/lib.rs` — the four organs as traits, so the interfaces are committed before any
  implementation is.
- `docs/experiments/E-001` — the first experiment, pre-registered.
- Everything else waits on E-001.
