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

## E-007 Amendment 1 (2026-09-29, before any labels exist): a real consumer for prime, from a window start

(This is E-007's own first amendment. "Amendment 1" elsewhere in this file means E-004's, which excludes grid-colony-one.)

**Why.** The pre-registration says the recall daemons "serve the observatory, OBC and the radio" (lines 13-14). That was never true. SpaceChild's survey of 2026-09-29 (11 repos, read-only, plus 25 minutes of copies of live requests) found this:
- The observatory recall panel and the radio DJ run the local `kannaka recall` CLI. It reads a local store and publishes nothing.
- The OBC citizens recall on `KANNAKA.substrate.recall`, not `KANNAKA.recall.<agent>`.
- Only two callers in the surveyed code reach `KANNAKA.recall.kannaka-prime`: the radio's OBC DM responder (at most 12 a day, and only when `RESPONDER_ENABLED=1`) and the command-center MCP recall tool, which operators use to probe. Hand-run `kannaka recall --remote`, `swarm brief --peers` and nats-CLI probes are a third path that no repo calls, and they are what the census saw.
- Day 7's census: prime has served 8 recalls in its life (seq 4948-5112, all 2026-09-28 UTC), every one an operator probe or rollout check, and none on day 7.
- The window's dominant recaller is not organic. On the O1 hub, connz shows `KANNAKA.recall.grid-colony-one` traffic coming from one python client named `kannaka-grid`. That is kannaka-grid on skywave querying its own colony mind on a timer: 12 requests in a 25-minute sample, 130-390 a day by the census (5-16 an hour averaged). E-004's Amendment 1 already excludes that agent. This records why that exclusion was right.

**Precondition (from Agent Flaukowski, 09-29).** The host serving prime runs a release containing kannaka-memory#1072, so that `.recall` events are not dropped silently (#1071). Before day 1 of the window this amendment applies to, one deliberate recall through the observatory path must show up as its `.recall` event on the bus. That probe's query hash is added to the operator hint table first, so it is tagged as an operator probe and can never be mistaken for the first organic event or start a window early (a probe is a `.recall` event, and a window opens the day after the first one).

**Change, effective at the start of the first window that opens after this amendment is merged AND kannaka-observatory#146 is deployed on the observatory host, confirmed by the precondition above. The current window is not touched.**
1. The observatory recall panel (`GET /api/hrm/recall` in server.js) sends its query to `KANNAKA.recall.kannaka-prime` with `kannaka recall --remote --agent-id kannaka-prime`. Among the surveyed consumers it is the only one whose queries are typed by people who are not probing it, so it cannot be the poller shape named under Known traps. If the request fails, it falls back to the local CLI and logs the fallback without the query text. Fallback recalls publish no event, so they are simply not counted.
2. The radio DJ stays on the local CLI. Its queries are semi-fixed: album names, plus the literal "consciousness resonance signal". Moving it would put exactly that poller into the ground truth.
3. The OBC citizens stay on `substrate`. Moving them would change what they do, not just where they send it.
4. The responder stays as it is. The report states whether `RESPONDER_ENABLED` was on during the window.

**Reporting additions. The decision rule, the guards, the window and the label rule are unchanged.**
- Recall events are tagged by caller class from a hint table the operator maintains (query hash -> class: observatory, responder, operator-probe). The event itself names no requester (kannaka-memory src/nats.rs:1705-1712), so anything not in the table is reported as unknown. The first table is `experiments/e004/hints/kannaka-prime-2026-09-29.json`: all eight of prime's recalls to date, tagged operator-probe.
- Operator probes still COUNT toward recalled-later. The hint table tags; it does not exclude. That is the registered rule, and this amendment keeps it.
- Kept-recalled is reported for all events and again with operator probes removed. The verdict is taken from all events, as registered. The second figure is shown so a reader can see how much of the ground truth was us.
- The ten-most-frequent `query_sha256` table (Known traps) is also split by caller class.

**Expected consequence, said in advance.** Human-typed volume on the observatory panel is low. The "at least 20 recalled-later events" guard may void the next window too, and the window then slides a week at a time as registered. A void run is the preferred outcome to inflating the ground truth with a fixed query.

**Lines 13-14 correction.** They are left as written, since the registration is append-only. This amendment is the correction.

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

**2026-09-28 (day 6): remember events can now come from every write path. This entry is plumbing only; the rules are unchanged.**

- kannaka-memory#1066 (7a7477a, closes #1057) makes every write path publish
  `.remember`: agent, chat, dream, absorb, sync, import, perception, as well as
  the CLI. This takes effect on each host once it runs a release that contains
  the change and its daemons have restarted. Non-CLI writes publish ids only:
  `memory_id` and `content_sha256`, with no text.
- kannaka-prime's `.recall` events are live from seq 5079 on, so prime is a
  serving agent. The remember side of the scored set therefore depends on
  prime's upgrade.
- `experiments/e004/export.py` resolves the text of ids-only events by
  `memory_id` from a read-only copy of the agent's store, taken at export time
  (`--store AGENT=PATH`). It uses the text only if its SHA-256 matches the
  event's `content_sha256`. It reports per agent how many events had inline
  text, how many were resolved, and how many were mismatched, missing, or had no
  store. Prime's content stays off the bus.
- The decision rule, guards, window, label rule and scored set are unchanged.

**2026-09-28 (day 6): test events leaked into the window, excluded before any labelling.**
While verifying kannaka-memory#1068, a manual check published four test `remember` events to the live bus under a made-up agent id, `e2e1067`, via cli, with content "alpha harbor", "beta pier", "gamma live" and "delta copy": KANNAKA_MEMORY_EVENTS seq 5097–5100, ts about 1790601983–1790601998. They are not real memories. That agent serves no recall, so it is outside E-007's scored set anyway, but the events sit in the shared world stream on a training day. The export therefore excludes the agent (`--exclude-agent e2e1067`, alongside Amendment 1's `grid-colony-one`). This entry is written before any labels exist and before the window closes. It is data hygiene for a known artifact, not a change to any rule.

**2026-09-29 (day 7): prime now has both halves of the plumbing, and almost no recall traffic.** Census: `experiments/e004/results/census-e007-2026-09-29.txt`. The rules are unchanged.

- kannaka-memory v0.16.13 is on every host (2026-09-28). kannaka-prime now publishes `.remember` through non-CLI paths, ids-only: `via` `research` (seq 5476–5478) and `dream` (seq 5564–5565). Its `.recall` events have been live since seq 5079. So the condition the day-3 entry named, a non-excluded agent that both publishes remembers and serves recalls, is met for the first time.
- The traffic is the problem now. Across the whole window, excluding `grid-colony-one`, prime served **8** recalls, all on 09-28, all distinct `query_sha256`, and all rollout and verification probes. SpaceChild served 2 test recalls. There were none on day 7. `grid-colony-one` (excluded) served 2,038.
- Those probes fall on training days, so they cannot label anything. Only held-out writes (days 21–30) are scored, against recalls in the 14 days that follow. They are recorded here so nobody mistakes them for organic traffic later.
- At the current rate, the guard "at least 20 recalled-later events" will void the run, and the window will slide. This is not an amendment. Whether organic recall traffic should reach prime at all, and from where (observatory, OBC, radio), is the author's decision. It is not settled here.
- Radio `hear` writes, which would be prime's `via` `perception`, are still denied at the hub pending a config reload (kannaka-memory#1074). When they arrive they are ids-only, about 3.5k per day, and separable on (`agent_id`, `via`).
- Method note: this census read the stream directly by sequence (`$JS.API.STREAM.MSG.GET`), seq 3211–5942 with no holes. The first `.recall` event matches the day-3 census exactly.

**2026-09-29 (day 7): E-007 Amendment 1 recorded before any labels.** The survey found no organic consumer of prime. The observatory panel will be routed to prime from a window start, once Nick approves and kannaka-observatory#146 is deployed. The current window runs as registered, and the recalled-later guard is expected to void it.
