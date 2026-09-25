# E-007: Does surprise keep what later gets recalled?

**Status:** pre-registered 2026-09-22. Waits on recall events reaching the bus (kannaka-memory#1038
publishes them; then a deployed daemon). Its window opens the
day after the first one appears.
**Decides:** the same question as E-004, whether prediction-error salience keeps more of
what later mattered than the retention policy alone, against a different and independent
ground truth: *a memory someone else later recalled*.

## Why a sibling and not an edit

E-004's ground truth is what settlement readings cite. That is a strong definition of
"mattered", and it depends on people filing readings. This one depends on nobody: the recall
daemons (`kannaka swarm serve`, `KANNAKA.recall.<agent>`) serve the observatory, OBC and the
radio, and from kannaka-memory#1038, each served recall publishes
`KANNAKA.events.memory.<agent>.recall` with the ids it returned, their similarities, and a
SHA-256 of the query (never the query or any content). It is a different ground truth with
different failure modes, so it is a different experiment with its own rule. The two share a
world stream and arms, and each is reported under its own rule; neither is picked after the
fact.

## Fixed choices, from E-004 unless named

- **World stream, predictor, surprise, arms U and S, seeds, cap, dreams:** exactly E-004's,
  including Amendment 1's exclusion of `grid-colony-one` and ADR-0040's operator with
  `Drive::Error` (`src/novelty.rs`).
- **Window:** 30 consecutive UTC days starting the day after the first `.recall` event is on
  the bus. Days 1–20 train the predictor; days 21–30 are absorbed by the arms.
- **Scored events:** held-out (days 21–30) `remember` events whose `agent_id` is an agent
  that published at least one `.recall` event during the window. A memory in a store that
  no daemon serves cannot be recalled, so counting it as "not recalled" would make it a
  negative by construction.
- **Ground truth:** a scored event is *recalled-later* if its `memory_id` appears in the
  `memory_ids` of `.recall` events from **at least two distinct `query_sha256` values**
  within 14 days after its `ts`. One question asked again and again is a poller, not two
  askers. Labels freeze 14 days after day 30; the list of recalled-later
  `(agent_id, memory_id)` pairs is hashed into the report.

## The measurement

After the last dream, per seed:

- **Kept-recalled:** of the recalled-later events, the fraction whose row survived the
  arm's forgetting (the parent or any of its facets). This is the primary number.
- **Precision of survivors:** of the rows that survived, the fraction that were
  recalled-later.
- **Φ** on E-001's shared instrument.

There is no recall@10 here. The recall event carries no query, by design, so there is no
question to probe with. Survival under the cap is the question "what to remember" in its
most direct form: the arm decided what to keep, and later use says what should have been
kept.

## Decision rule

- **Kept** if arm S's kept-recalled exceeds arm U's with the Welch 95% interval excluding
  zero, *and* precision of survivors rises, *and* Φ does not fall with the interval excluding
  a drop of more than 0.02. Counts toward ADR-0002's salience path going live; if E-004
  also runs, the report states both verdicts.
- **Declined** if S's kept-recalled is below U's with the interval excluding zero, or if
  precision falls.
- **Undecided** otherwise: the arms rerun at 20 seeds before anything ships. If the rerun is
  still undecided, the verdict is **declined for now**: the salience path does not go live
  on an experiment that could not tell. (E-003 showed what an undecided branch with no exit
  does.)

## Guards, checked before a run is scored

- E-004's guards: the predictor beats the last-state baseline, the collapse guard holds, a
  constant stream gives surprise exactly 0, and both arms forgot at least three quarters of
  what they absorbed.
- **At least 20 recalled-later events** among the scored ones, or the run is void and the
  window slides forward a week at a time.
- **Geometry.** Recall picks memories by the serving store's encoder. If that is the arms'
  encoder (E-001's `mxbai-embed-large`), then which rows got recalled and which rows an arm
  keeps can share a geometry, and a win can be the encoder agreeing with itself. The report
  states each serving store's encoder stamp; a run where any is the arms' encoder is flagged
  and **cannot be Kept** on its own.
- The label set was frozen before the arms ran (hash in the report).

## Known traps

- **Pollers.** The distinct-hash rule stops one scheduled query counting twice. It does not
  stop two pollers with different fixed queries both hitting the same popular memory. The
  report lists the ten most frequent `query_sha256` values and their share of all recall
  events, so a reader can see how much of the ground truth is automation.
- **Recency.** Recall ranks favour recent memories, and surprise may correlate with recency.
  The report gives kept-recalled by write-day for both arms, so a recency effect is visible
  rather than mistaken for salience.
- **Self-recall.** A daemon recalling its own agent's memories on behalf of a local tool is
  still a use; the event does not distinguish askers, and this experiment does not pretend it
  can.

## Log

Append-only. Nothing here changes the decision rule, the guards or the scored set.

**2026-09-25 (day 3): the window is open and, as things stand, every run will be void.**
Census: `experiments/e004/results/census-e007-2026-09-25.txt`.

- The first `.recall` event landed 2026-09-22T22:47:19Z, so by the rule above day 1 is
  2026-09-23, day 30 is 2026-10-22, and labels freeze 2026-11-05.
- All 997 `.recall` events so far come from `grid-colony-one`, which Amendment 1 excludes.
  Every other daemon was denied publishing the event by the `serve` user's NATS ACL. That
  fix is kannaka-memory#1056, and it takes effect at the hub's next config reload.
- Even after the reload, the scored set also needs `remember` events from the agents that
  serve recalls. Only the `kannaka remember` CLI publishes those. In days 1–3, the 21
  remember events came from two agents (`agent-1a6bb2ee`, `agent-acd3e2ed`), and neither
  serves recall.
- The guard "at least 20 recalled-later events, or the run is void and the window slides
  forward a week at a time" already covers this, so no amendment is made. For a window to
  qualify, some non-excluded agent must both publish remembers and serve recalls. Which
  agent should do that, and through which path, is the author's decision. It is not settled
  here.
