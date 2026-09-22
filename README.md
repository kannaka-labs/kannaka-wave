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
| Conscience | rails between the voice and every effector: a hashed charter, the Five Refusals per effector, a hash-chained audit, and packages for a person to sign, never an action. The reasoner is never the arbiter | kannaka-steward |
| Architecture | the whole described in KannakaHDL (`architecture/wave.khdl`), resolved against a registry built from the record, which refuses when a faculty has no honest answer and names it | kannaka-hdl mind domain |

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
- `src/conscience.rs`, `src/sha256.rs`, `tests/no_effector.rs` — **the conscience**:
  the steward's rails, charter and audit chain in Rust. The voice names an effector and
  a confidence; the charter says what that effector weighs and which of the Five
  Refusals it is cleared for; the rails refuse, escalate, or emit a package for a
  person to sign, recorded in a SHA-256 chain that refuses to be decided against once
  tampered with. A test scans `src/` so that nothing can sign, spend, spawn a process,
  or open a connection outside the one HTTP client. `wave propose | charter | audit`;
  an example charter is in `charters/`. `wave ask --propose` lets the voice propose an
  action after answering: it is told effector names and one-line descriptions, nothing
  else, and whatever it proposes goes through the rails and onto the record.
- `architecture/wave.khdl`, `architecture/registry.py` — **the architecture**: the
  whole as a KannakaHDL program, eight faculties across the organs, resolved against a
  registry built from this repository's own evidence (measured numbers from run
  experiments; executed tests where no experiment is registered; nothing for an unrun
  one). In strict mode it refuses today and names Propose, Predict and Surprise, which
  are E-008, E-004 and E-007.
- `docs/experiments/E-001` — whether the waves earn their keep, against a plain vector
  store with the same encoder, facets and forgetting policy. **Run; the waves lose**
  (recall@10 on zero-overlap probes 0.24 against 0.58, Φ equal, ten seeds). The
  harness, the raw rows and the report are in `experiments/e001/`.
- `docs/lineage/` — what that decision moved out of the build, kept whole: the
  `10000.00001` number system as a tested type.
- `docs/experiments/E-003` — the bridge operator at the conscience boundary.
  **Declined** on a pre-run check: the registered residue is the constant 0.190983 for
  every action (`experiments/e003/`).
- `docs/experiments/E-004` — whether surprise picks what to remember. Pre-registered.
  A pre-run check found no ground truth on record, so it was amended before its window
  opened: labels are verified bus citations, and the window runs 2026-09-23 to 10-22. The
  salience operator it needs is `src/novelty.rs`.
- `docs/experiments/E-007` — the same arms, scored against what peers later recall.
  Pre-registered; it waits on recall events reaching the bus.
- `docs/experiments/E-008` — E-003's sentence with the real rails and voice: does
  charter-then-plan end where plan-then-charter ends, and does the disagreement separate
  what a person would endorse better than the rails' own verdict? Pre-registered;
  `experiments/e008/` runs it.

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
