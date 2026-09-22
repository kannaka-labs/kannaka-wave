# The architecture organ: `wave.khdl`

ADR-0001 §Architecture: *the whole is described in KannakaHDL. `wave.khdl` grows a
Wave from eight faculties across the four organs and refuses, in strict mode, when a
faculty has no honest answer. An unresolved faculty is demand and names what to build
next.*

| file | role |
|---|---|
| `wave.khdl` | the program: a Wave splits into four organs, each into two faculties, each a `mind.faculty` query with floors taken from numbers the record already committed to |
| `registry.py` | builds `wave-registry.json` from this repository's evidence: experiment reports for measured faculties, the crate's own tests (executed one by one) for faculties with no experiment registered, and **no row** for a faculty whose experiment is registered but unrun |
| `wave-registry.json` | the built registry, committed so the plan is reproducible; rebuild it after any experiment lands |
| `wave-plan-2026-09-22.json` | the plan grown in speculative mode on the day the organ was written: five faculties resolved, three `capability_discovery` requests. The dated record of what the Wave lacked. CI rebuilds the registry from the evidence and fails if the committed one is stale or any contract fails when executed |

```sh
python3 architecture/registry.py --out architecture/wave-registry.json
kannaka-hdl grow architecture/wave.khdl --mind-registry architecture/wave-registry.json --unresolved strict
kannaka-hdl grow architecture/wave.khdl --mind-registry architecture/wave-registry.json --emit json
```

`kannaka-hdl` is [kannaka-labs/kannaka-hdl](https://github.com/kannaka-labs/kannaka-hdl)
(`cargo build --release` there; mind domain since v0.10).

## The eight faculties, and where each one's answer comes from

| organ | faculty | floor in `wave.khdl` | evidence |
|---|---|---|---|
| substrate | Recall | persistence ≥ 0.5, `zero_overlap_recall` | E-001, arm V recall@10 on zero-overlap probes: **0.576** |
| substrate | Forgetting | persistence ≥ 0.75, the store's two retention tests | E-001 arm V: rows forgotten per row injected, and the tests, run |
| voice | Speak | persistence ≥ 0.808, evidence ≥ 2, `faithfulness_floor` | E-005 anchored faithfulness **0.839**, CPU replication committed; the floor is the adoption rule's own |
| voice | Propose | persistence ≥ 0.5, `commutes_with_the_charter` | **E-008, registered, not run: no row** |
| conscience | Decide | persistence = 1.0, six named tests | no experiment registered (E-003 declined; the rails kept their tests): the tests, run |
| conscience | Audit | persistence = 1.0, four named tests | same |
| world | Predict | persistence ≥ 0.5, `beats_last_state_and_the_mean` | **E-004, registered, not run: no row** |
| world | Surprise | persistence ≥ 0.5, `keeps_what_later_mattered` | **E-004 / E-007, registered, not run: no row** |

So today, in strict mode, the Wave does not grow, and the refusal names three
faculties: Propose, Predict, Surprise. Those are E-008, E-004 and E-007, which is the
build order the record already implies. In `speculative` mode the plan grows with five
resolved faculties and three `capability_discovery` requests, one per demand.

## What keeps this honest

- **No number is typed in.** Persistence comes from a committed report line or from a
  test that passed when `registry.py` ran; a parser that cannot find its line stops the
  build rather than defaulting.
- **A test is a contract only when executed.** `registry.py` runs each named test with
  `cargo test <name> -- --exact` and records the result; a renamed or deleted test is a
  failed capability, and the floor refuses it.
- **An unrun experiment is absence, not zero.** A faculty waiting on its experiment has no
  row at all. Writing a row with persistence 0 would let a lower floor "resolve" it; the
  organ's whole purpose is that it cannot.
- **The plan names its evidence.** kannaka-hdl records the registry snapshot and its hash
  in every plan, so a grown Wave says which measurements it stands on.

## Limits, stated

- The eight faculties are this repository's reading of ADR-0001's "eight faculties across
  the four organs" (ADR-0002's world organ supplies two; the architecture is the program
  itself). Recorded in ADR-0001's status of work.
- "Forgetting" has no single experimental number of its own; E-001 measured that the
  policy deletes, and the store's tests say how. Its persistence is the forgotten-per-
  injected ratio clipped at 1, which is coarse, and says so here.
- The mind domain has no notion of a coupling between faculties that this repository has
  measured, so `wave.khdl` declares none. A bridge would be a claim.
