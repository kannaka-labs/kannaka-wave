# kannaka-wave

An AI built from the ground up on the Holographic Resonance Medium, with the language
model as an organ rather than a layer. A v2 of eighteen months of experiments, started
from scratch, with every idea traced to where it came from and what was measured about
it.

## Read in this order

1. [`docs/adr/ADR-0001-kannaka-wave.md`](docs/adr/ADR-0001-kannaka-wave.md) — the
   decision: four organs, one loop, and the one experiment that runs before anything
   else is built.
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
| Substrate | a chiral holographic medium that decides what persists; encoded by the voice's embeddings, written as atomic facets, dreamed with triage | kannaka-memory ADR-0020/0021/0024/0049/0054 |
| Voice | an open-weight model on her own words, stateless, reading the substrate through `recall(question)` and entering it only in the dream | kannaka-memory ADR-0057/0058, rogue-agent |
| Conscience | rails between the voice and every effector; the reasoner is never the arbiter | kannaka-steward |
| Architecture | the whole described in KannakaHDL, which refuses when a faculty has no honest answer | kannaka-hdl mind + code domains |

## The loop

```
experience → facets → substrate → dream (consolidate, propose, forget)
    → export her words → judge (controls) + external verdicts → LoRA → serve
    → wave.khdl: whole, or the demand by name
```

## What is in the first commit

- `src/chiral_scale.rs` — the `10000.00001` number system as a real, tested type. It was
  pseudo-code in ADR-0021 for six months.
- `src/lib.rs` — the four organs as traits. Interfaces are committed before
  implementations.
- `docs/experiments/E-001` — whether the waves earn their keep, against a plain vector
  store with the same encoder, facets and forgetting policy. The substrate is not
  written past that point until this has an answer.

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
