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
| `export.py` | A bus dump becomes `events.jsonl`. Text for ids-only remember events (kannaka-memory#1066) is resolved by `memory_id` from a store copy and verified by `content_sha256`. Tests are in `test_export.py`. |
| `labels.py` | The E-007 and E-004 ground truths, each frozen with a hash. |
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

## From the bus to the harness

```sh
# 1. the world stream: one or more query_messages dumps of KANNAKA.events.memory.>
python3 export.py --dump mem-a.json --dump mem-b.json --day1 2026-09-23 --days 30 \
    --exclude-agent grid-colony-one --exclude-agent e2e1067 \
    --store kannaka-prime=/path/to/prime-store-copy \
    --out events.jsonl        # summary on stderr; the dump and events.jsonl stay out of git
# 2. the labels
python3 labels.py e007 --events events.jsonl --recalls recall-dump.json --out labels-e007.json
python3 labels.py e004 --events events.jsonl --readings ../../../assay/readings --dump mem-all.json \
    --out labels-e004.json --probes probes.json
# 3. embed, train, run, report
harness/target/release/e004-harness prepare --events events.jsonl --cache vectors.bin --probes probes.json
harness/target/release/e004-harness run --events events.jsonl --cache vectors.bin --arm U --seed 1 --cap N \
    --labels <(python3 -c 'import json;print(json.dumps(json.load(open("labels-e007.json"))["labels"]))') --probes probes.json --out runs/U-1.json
python3 report.py --runs runs/
```

### Ids-only remember events: resolving the text from a store copy

Since kannaka-memory#1066 every write path publishes
`KANNAKA.events.memory.<agent>.remember`, not only `kannaka remember`. Only the
CLI (`via=cli`) carries `content` by default. Every other origin (agent, chat,
dream, absorb, sync, import, perception, ...) publishes at the `ids` level:
`memory_id`, `agent_id`, `importance`, `modality`, `via`, `content_sha256`, and no
text, because the memory lane is readable by `anon`. kannaka-prime's writes are
all of this kind, and its memories are not published in clear on purpose. So the
exporter resolves the text by `memory_id` from a copy of the agent's store, on
the export host, at export time:

1. On the host that serves the agent, take a read-only copy of its store dir at
   export time. For kannaka-prime on O1, that is `/home/opc/.kannaka`. Copy it when no dream or save is running; if the copy fails to load, take it again. At least
   `kannaka.hrm` (and `.encoder`) is needed: `cp -a /home/opc/.kannaka/kannaka.hrm
   /home/opc/.kannaka/.encoder /tmp/prime-store-copy/`. Never pass the live dir;
   the exporter refuses the current user's `~/.kannaka`.
2. Run `export.py` there with `--store kannaka-prime=/tmp/prime-store-copy`
   (repeat `--store` once per agent). Use `--kannaka /path/to/kannaka` if the
   binary is not on `PATH`; it must be able to read that store, so use the host's
   own binary. The exporter runs `kannaka export-json --slim` once per store,
   builds an id → text map, and uses it only for events that lack `content`.
   It runs kannaka with `KANNAKA_READONLY=1` and remember events off, against a
   scratch data dir that holds only a symlink to the copy's `.hrm`. The scratch
   dir is there because a store's `config.toml` carries an absolute `hrm.path`,
   and the CLI prefers that over `KANNAKA_DATA_DIR`. Pointing kannaka at the
   copy directly would read the live store.
3. Alternatively, save that command's stdout on the host
   (`KANNAKA_DATA_DIR=<scratch dir> KANNAKA_READONLY=1 kannaka export-json --slim
   > store-kannaka-prime.json`) and pass `--store-json kannaka-prime=store-kannaka-prime.json`.
   That file is memory content: it stays out of git, and so does `events.jsonl`,
   which now holds resolved text (both are in `.gitignore`).
4. Delete the store copy and any `store-*.json` once `events.jsonl` is written.

**Verification.** A resolved text is used only if the SHA-256 of its UTF-8 bytes,
untrimmed, equals the event's `content_sha256`. That is kannaka's own
`remember_events::content_sha256`, computed over the stored content. This check
guards against a store copy that has drifted from the event, for example a
memory rewritten after it was published, or a copy of the wrong store. Such an
event is counted as `hash_mismatch` and left out; it is never given the wrong text.

**The summary** (stderr) reports, per agent, where every remember event in the
window got its text: `inline` (the event carried it), `resolved_by_id`,
`hash_mismatch`, `unresolved_missing` (the id is not in the copy: pruned,
forgotten, or the copy predates the event), `skipped_no_store` (ids-only, and no
store given for that agent), `unverifiable_no_hash`, and `empty_text`. Anything
outside the first two triggers a warning line. Nothing is dropped silently.
Each exported event carries `text_source: "inline" | "store"`.

This is plumbing. It changes which events have text, not any decision rule,
guard, window or label. The tests are in `test_export.py`:
`python3 -m unittest discover -s experiments/e004 -p 'test_*.py'`. They use a
stub `kannaka`; set `E004_REAL_KANNAKA=/path/to/kannaka` to add a round trip
through a real binary on a throwaway store in a temp dir.

### Recall events and `caller_class` (E-007)

`export.py --recalls-out recalls.jsonl` also writes the
`KANNAKA.events.memory.<agent>.recall` events in the dumps, one row per line
(`key, agent, ts, day, query_sha256, top_k, memory_ids, similarities, via`, no
query and no content, exactly what `swarm serve` publishes), with a
`caller_class` column appended: `observatory`, `responder`, `operator-probe` or
`unknown`. No day window applies to these rows, since the E-007 label window
runs 14 days past day 30. The summary carries per-agent counts per class
(`recalls.callers`), and stderr gets one `<agent> recalls: ...` line per agent.

Its limits are the event's. The payload does not name the requester, so the
class is read off `top_k`, the one field the requester chooses: `1` or `2` is
`operator-probe` (the hand-sent rollout probes; no automated caller sends
these), and so is `10`, the command-center MCP `recall` tool's default, on the
grounds that only an operator drives that tool (a caller who sets topK to 10
by hand would be misfiled; this is the one defeasible rule). Everything else is `unknown` by default, and `top_k = 5` in particular
is a three-way collision that `agent_id` cannot break: the radio responder
sends 5, `kannaka recall --remote` (the observatory's call) defaults to 5, and
`swarm brief --peers` sends 5. `10` is the MCP recall tool's default and has no
class here. So `observatory` and `responder` are only ever assigned through
`--caller-hints hints.json`, a JSON object `{query_sha256: class}` for an
operator who knows the query text (the observatory's fixed question, or their
own probes): hash the query string exactly as sent, SHA-256 of its UTF-8 bytes
(`content_sha256()` in `export.py` computes the same thing; the responder's
string is `"<sender>: <dm text[:300]>"`), and tag it. A hint wins over the
heuristic. The classifier is `caller_class()` in `export.py`, and its docstring
is the record of which requester sent what on 2026-09-29; when a requester
changes its `top_k`, that table is what to update. The column is descriptive:
it changes no label, rule or guard.

Each label file carries the sha256 of its sorted keys, which the report records
as the frozen set. `export.py` prints the per-day counts and what the exclusion
removed; `labels.py e007` lists the ten most frequent query hashes and their
share, for the poller trap; `labels.py e004` lists every citation's verdict.

## Inputs, when the windows close

- `events.jsonl`: the world stream, one event per line in bus order,
  `{"key": "<subject>#<seq>", "agent", "day": 1..30, "text"}`, with
  `grid-colony-one` already excluded (Amendment 1). Ids-only events have their
  text resolved from a store copy (above). The raw export and `events.jsonl`
  are memory content and are not committed. The census file shows what an export's aggregates look like.
- `labels-e007.json`: keys recalled later, from `.recall` events, two distinct
  query hashes within 14 days. `labels-e004.json` / `probes.json`: verified
  citations (`bus_cite.py`, `VERIFIED` only) and their settlement-side
  questions.
- `vectors.bin`: `prepare`'s cache. `mxbai-embed-large` at 1024-d, E-001's
  encoder, over ollama.

`python report.py --runs <dir>` applies each experiment's decision rule verbatim
and writes `report.json` beside the runs.
