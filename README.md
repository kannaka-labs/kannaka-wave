# kannaka-wave

An AI built from the ground up on the Holographic Resonance Medium, with the language
model as an organ rather than a layer. A v2 of eighteen months of experiments, started
from scratch, with every idea traced to where it came from and what was measured about
it.

## Read in this order

1. [`docs/adr/ADR-0001-kannaka-wave.md`](docs/adr/ADR-0001-kannaka-wave.md) — the
   decision: four organs, one loop, and the one experiment that runs before anything
   else is built. ADR-0002 adds a fifth organ, the world.
2. [`docs/archaeology/README.md`](docs/archaeology/README.md) — the dig. Every concept
   from the constellation (the `dx/dt` equation, the chiral hemispheres, the
   `10000.00001` number system, the bridge operator `Ξ = [R, G]`, belief spiral math,
   Fano folds, facets, triage, the steward, the trust model, the autoresearch loop)
   with its source, its measurement, and a verdict: bring, transform, measure, or leave.
3. [`docs/experiments/`](docs/experiments/) — pre-registered experiments with their
   decision rules written before the run.

## The four organs

| organ | what it is | comes from |
|---|---|---|
| Substrate | decides what persists: the voice's embeddings, atomic facets, a stated forgetting policy, in a plain vector store. E-001 (2026-09-09) measured the chiral wave medium against exactly that and the medium lost; it is in `docs/lineage/` | kannaka-memory ADR-0049/0054; E-001 |
| Voice | an open-weight model on her own words, stateless, reading the substrate through `recall(question)` and entering it only in the dream | kannaka-memory ADR-0057/0058, rogue-agent |
| Conscience | rails between the voice and every effector; the reasoner is never the arbiter | kannaka-steward |
| Architecture | the whole described in KannakaHDL, which refuses when a faculty has no honest answer | kannaka-hdl mind + code domains |

## The loop

```
experience → facets → substrate → dream (consolidate, propose, forget)
    → export her words → judge (controls) + external verdicts → LoRA → serve
    → wave.khdl: whole, or the demand by name
```

## What is here

- `src/lib.rs` — the organs as traits. Interfaces are committed before implementations.
- `src/store.rs`, `src/facet.rs`, `src/encoder.rs` — **the substrate**, written to
  E-001's verdict: a vector store with arm V's measured semantics (cosine, facets
  resolved to parents, one recall per family, promotion at three hits, per-class caps
  and time-to-live), the ADR-0049 decomposer ported from kannaka-memory, and an ollama
  encoder over `std::net`. Persistence is versioned and checksummed; a newer file is
  refused, never guessed at. Still no dependencies.
- `src/voice.rs`, `src/adoption.rs`, `src/bin/wave.rs` — **the voice**: stateless
  over ollama, querying the substrate only with a pure reduction of the prompt to its
  question, entering it only through a gated dream proposal, replaced only when a
  judge with controls and an external evaluator agree. `wave remember | ask | dream |
  status` runs it from a shell.
- `docs/experiments/E-001` — whether the waves earn their keep, against a plain vector
  store with the same encoder, facets and forgetting policy. **Run; the waves lose**
  (recall@10 on zero-overlap probes 0.24 against 0.58, Φ equal, ten seeds). The
  harness, the raw rows and the report are in `experiments/e001/`.
- `docs/lineage/` — what that decision moved out of the build, kept whole: the
  `10000.00001` number system as a tested type.
- `docs/experiments/E-003`, `E-004` — the bridge operator at the conscience boundary,
  and whether surprise picks what to remember. Pre-registered, not yet run.

```sh
cargo test
```

No dependencies. It compiles and tests with nothing downloaded.

## The rules this repository is bound by

From the constellation's feedback record, carried into ADR-0001 as binding:

- A check must be at least as strong as what it checks.
- A claim of absence needs playback evidence, never a search the claimant scoped.
- Nothing is called a belief without its three predictions.
- Identity comes from configuration, never from the process that happens to be running.
- Every experiment pre-registers its decision rule; ten runs, not five.

## License

Space Child License v1.0.
