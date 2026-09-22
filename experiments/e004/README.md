# E-004 / E-007 harness — does surprise pick what to remember?

The specs are [`E-004`](../../docs/experiments/E-004-does-surprise-pick-what-to-remember.md)
and [`E-007`](../../docs/experiments/E-007-does-surprise-keep-what-gets-recalled.md).
This directory is the machinery that will run them once their windows close
(E-004: labels freeze 2026-10-30; E-007: 14 days after its day 30). It was built
and self-tested before either window opened, so that the only new input on the
day is data.

## Layout

| file | role |
|---|---|
| `harness/` | Rust. Links `kannaka-wave` (the substrate E-001 promoted, the facet decomposer, the encoder client, ADR-0040's operator with `Drive::Error`) and `consciousness-core` (E-001's Φ instrument). `selftest` runs the guards on synthetic streams. `prepare` embeds every event, facet and probe once into one cache. `train` fits the predictor and applies the adoption rule. `run` is one seed of one arm: absorb days 21–30 with a dream per day, then the E-007 survival numbers, the E-004 recall@10 numbers, and Φ. |
| `report.py` | Means, standard errors, Welch intervals, the guards, and each experiment's decision rule applied verbatim (E-004 with Amendments 1 and 2; E-007 with its exit from the undecided branch). Checked against fixture runs shaped Kept, Declined, Undecided, Void and guard-failed. |
| `results/census-2026-09-22.txt` | The bus census that found the retired window and the missing ground truth. |

The product crate (`kannaka-wave` at the repo root) has no dependencies by
decision. This harness is not the product; it is the instrument.

```sh
cd experiments/e004/harness && cargo build --release
./target/release/e004-harness selftest
```

## Fixed choices (not in the spec, recorded here)

| choice | value | why |
|---|---|---|
| what a seed seeds | the predictor's weights and minibatch order | the spec says a seed "seeds only the dream", but `VectorStore::dream_at` has no randomness, so that seeds nothing. Arm U is therefore identical across seeds; the interval in the report is arm S's variation. Said in the report. |
| surprise operator state | fresh at day 21, keyed by `agent_id` | the spec fixes the operator and not its warm-up. Running it over days 1–20 would feed it the predictor's error on its own training data, a baseline biased low, so day 21 would look surprising by construction. |
| importance | U: 0.5 uniform. S: `0.5 · min(1 + score, 3)` | the spec's "default scaled by `1 + surprise`, clipped at 3×"; `score` is the operator's directional output, not the raw distance |
| retention class | `""` (every parent) | the wave store matches a class by text prefix; an empty prefix is "the stream's content class" without touching the text the encoder sees |
| cap | on the command line, in each run's JSON | chosen from the held-out count so that at least three quarters is forgotten; the guard checks that it was, not that it was meant to be |
| forgetting order | the store's own: least-recalled, then least important, then oldest (insertion order) | E-004 says "oldest-and-least-recalled among the least important"; with no recalls between absorb and dream, importance decides, and the two orders agree |
| predictor | k = 8, hidden 512, Adam, lr 1e-3, batch 32, 30 epochs | k, hidden and the loss are the spec's; the optimiser and epochs are not, and are in every run's JSON |
| Φ | k-NN (k = 8) cosine graph over surviving parents, spherical k-means (8), `consciousness_core::iit::compute_phi` | E-001's instrument, same method |
| bootstrap | 1,000 resamples, percentile 95%, paired per held-out event | |

## What the selftest found before any data existed

The guards were run on synthetic streams (16-d, k = 4, 600 events, 20 per
"day") on 2026-09-22. Five passed. One failed in a way that is about the
pre-registration, not the code:

**On pure noise the predictor was adopted.** It beat the last-state baseline
(held-out MSE 0.0988 against 0.1236) with the interval excluding zero, and the
collapse ratio was 0.58, above the ⅓ floor. On i.i.d. unit vectors the
last-state predictor's error is about 2/D and a constant predictor's about
1/D, so anything that drifts toward the mean "beats the baseline" while
knowing nothing about the world, and a half-collapsed predictor clears a
floor of ⅓.

The fix is not a different floor (that would be tuning the guard to the case)
but a stronger baseline, and it is E-004 **Amendment 2**, made before the
window opened: the predictor must also beat the constant predictor (the mean
of the training targets) with a paired bootstrap interval excluding zero. On
the noise stream it does not (0.0988 against 0.0628); on the predictable
stream it does (0.0015 against 0.0589). The collapse floor stays at ⅓.

## Inputs, when the windows close

- `events.jsonl`: the world stream, one event per line in bus order,
  `{"key": "<subject>#<seq>", "agent", "day": 1..30, "text"}`, with
  `grid-colony-one` already excluded (Amendment 1). The export script is
  written when the window closes; the raw export is memory content and is not
  committed. The census file shows what an export's aggregates look like.
- `labels-e007.json`: keys recalled later, from `.recall` events, two distinct
  query hashes within 14 days. `labels-e004.json` / `probes.json`: verified
  citations (`bus_cite.py`, `VERIFIED` only) and their settlement-side
  questions.
- `vectors.bin`: `prepare`'s cache. `mxbai-embed-large` at 1024-d, E-001's
  encoder, over ollama.

`python report.py --runs <dir>` applies each experiment's decision rule verbatim
and writes `report.json` beside the runs.
