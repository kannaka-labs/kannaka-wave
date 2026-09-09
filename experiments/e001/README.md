# E-001 harness — do the waves earn their keep?

The spec is [`docs/experiments/E-001-do-the-waves-earn-their-keep.md`](../../docs/experiments/E-001-do-the-waves-earn-their-keep.md).
This directory is the machinery that runs it. Everything that was a choice is
recorded here, so the report can say what was run rather than what was meant.

## Layout

| file | role |
|---|---|
| `harness/` | Rust. Links `kannaka-memory` (the production substrate) and `consciousness-core`. `wave-build` / `wave-run` drive **arm W** through `KannakaMemorySystem`, the same object the CLI wraps. `facets` emits the decomposition arm V must mirror. `phi` is the one Φ instrument for both arms. `diag` prints what the triage stage would see. |
| `arm_v.py` | **Arm V**: a flat vector store with the same encoder, the same facets, the same forgetting policy, and nothing else. |
| `run.py` | Orchestrator. `--prepare` embeds every text once in batches; then ten seeds × two arms → `results.tsv`. |
| `report.py` | Means, standard errors, Welch intervals, and the pre-registered decision rule applied verbatim. |

The product crate (`kannaka-wave` at the repo root) has no dependencies by
decision. This harness is not the product; it is the instrument.

## Fixed choices (not in the spec, recorded here)

| choice | value | why |
|---|---|---|
| distractor churn per cycle | 40 | 30 cycles × 40 = 1,200 injections against a cap of 60, so the policy forgets ~1,140 rows: forgetting is exercised, not assumed |
| retention table | `"distractor:" = { cap = 60 }` | corpus rows carry no prefix and are never eligible; identical in both arms |
| distractor pool | Nick's local agent store, 1,572 memories, minus any content equal to a corpus row | real memories from a different life, prefixed `distractor: ` |
| recall knobs | **production defaults** (nothing set) | see "What the smoke runs found" |
| Φ instrument | k-NN (k=8) cosine graph over survivors' embeddings, k-means (8, fixed seed) partition, `consciousness_core::iit::compute_phi` | one function, both arms, computed from the same vectors; arm W's native `observe` Φ is recorded separately and not compared |
| embeddings | `mxbai-embed-large`, 1024-d, one cache keyed by exact text, filled in batches of 64 | the store embeds one text at a time and ollama on this CPU takes 2.85 s each that way against 0.32 s batched; the vectors are identical either way |

## What the smoke runs found before any seed ran

The spec says both arms' triage must be shown to delete at least one memory
before a real run is scored. That guard caught two things in the production
substrate, and a third fell out of running recall on a dreamed store.

**1. A fresh store never writes facets.** `HrmStore::new` builds a flat medium
(`chiral: None`), and the flat `absorb` path does not decompose; the CLI has the
same shape and is chiral only after its first reload. A 30-row build produced
30 rows. The harness now calls `upgrade_to_chiral()` before the first row: 63.

**2. ADR-0054's retention triage does not persist through a dream.** Stage 6b'
ghosts by setting the cache's `amplitude = 0.0`. The dream ends with
`rebuild_cache()`, which reads amplitude back from the medium's energy, so every
ghost revives before it can be observed. Measured on the smoke store with
`cap = 3`: eleven eligible distractors, the dream reporting "25 pruned", zero
rows at amplitude 0 afterwards, eleven distractors still live. The `diag`
command exists to show exactly this. The harness applies the stage's own
predicate and ordering through `triage_forget`, which deletes from both
hemispheres. With that, `cap = 3` leaves three. Filed upstream as a kannaka-memory issue.

**3. The eval knobs do not survive dreaming.** The 2026-08-02 encoder run and
the Harbor evals set `KANNAKA_RECALL_ENERGY_EXP=0.0` (energy-neutral ranking)
on an *undreamed* store. On a dreamed store that knob lets dream-created rows
(`__consolidation_summary_layer_0`, `[cross-cluster hallucination] …`) outrank
every real memory at similarity 1.0. Under production defaults they do not.
E-001 runs with production defaults, which means arm W's recall here is not
directly comparable to the 0.62 on record; the report says so.

**4. The production deep dream does not scale past about a thousand rows.**
`KannakaMemorySystem::dream()` is five phases, and phase 2 is the legacy
particle consolidation: `stage_detect` and its siblings, pairwise over every
row in 10k dimensions, the pipeline ADR-0022 diagnosed as "waves snap to
particles when observed". At 1,950 rows one dream ran for more than 26 minutes
of a core on debain2 without finishing. Thirty per seed on a store that grows
to 4,000 rows is not runnable, and it is not the waves. Arm W therefore dreams
phases 1, 3 and 4 — `dream_native` (3 cycles, temperature 1.0, the engine's
default chiral perturbation 0.0), callosal Kuramoto coupling, the lite chiral
pass — which is exactly the "annealing + Ξ cross-callosal step" the spec names.
Every run's JSON records this; `--dream deep` restores the full dream.

Also on the write path: `remember_with_category` flushes the whole store to
disk after every row, 150–200 MB at this size. Invisible in production, where
the CLI absorbs one row per process; 2.6 s per row in a loop. The harness
absorbs through the engine's store directly and saves once.

**5. Facets cost the medium a third of its recall; they cost cosine nothing.**
Build-only, no dreams, 83 probes, the same frozen corpus (`diag.sh`):

| variant | p50 hits /50 | z33 hits /33 |
|---|---|---|
| medium, production knobs, facets on (arm W's undreamed baseline) | 15 | 12 |
| medium, energy-neutral knob, facets on | 20 | 9 |
| medium, production knobs, facets off | 24 | 15 |
| the 2026-08-02 record: medium, energy-neutral, facets off | 31 | 22 |
| plain cosine with facets, after two dream cycles (arm V smoke) | 37 | 19 |

Read-side resolution is on and works (every arm-W top-10 slot mapped to a
corpus parent, no duplicates); the loss is on the write side. A parent and its
facets are absorbed as three or four near-duplicate waves that interfere with
each other, and "storing is thinking" cuts both ways. Cosine has no
interference, so the same facets cost it nothing. The pre-registered arms both
use facets, as the spec says; `E001_NO_FACETS=1` runs a labelled W0/V0 pair for
attribution, not for the decision.

One more property, mirrored rather than fixed: **facets outlive their parents
under triage.** A distractor's facets are stored as plain rows whose text does
not start with `distractor:`, so the retention rule never matches them. Both
arms behave identically, so the comparison holds, but survivor counts include
facets of forgotten parents.

## Running it

```sh
# once: build the harness against the substrate, sharing its build cache
cd harness && CARGO_TARGET_DIR=../../../../kannaka-memory/target cargo build --release

# once: the corpus and pool exports, the probe files, then the embedding cache
python run.py --work <work> --prepare

# the runs (hours for arm W; arm V is minutes), then the report
python run.py --work <work> --seeds 1..10
python report.py --work <work>
```

`<work>` holds `corpus-slim.json` (the frozen 615-memory snapshot, sha
`339e7ad9…`, exported with `kannaka export-json --slim`), `local-slim.json`
(the distractor pool), the four probe/expected files from the Harbor evals, and
`targets-z33.json` for the zero-overlap invariant check.
