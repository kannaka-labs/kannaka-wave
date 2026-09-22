# E-004: Does surprise pick what to remember?

**Status:** pre-registered; unblocked 2026-09-09 when E-001 promoted the vector store.
**Pre-run check 2026-09-22: blocked on ground truth.** The world stream exists; the
"load-bearing" labels the decision rule is scored on do not, in any source on record. The
salience operator is built (`src/novelty.rs`) with a sign correction. Nothing has run.
See §Pre-run check.
**Decides:** whether a latent world model's prediction error, as ADR-0002's salience
signal, keeps more of what later mattered than a stated retention policy alone.

## The question

ADR-0002 says the agent should preferentially remember where its model of reality was
wrong. That is a claim about retention under pressure: with a cap on what survives, does
surprise-weighted importance keep the rows that later questions need? The alternative
hypothesis, which the record supports at least as well, is that the stated retention
policy plus recall reinforcement already keeps them, and surprise keeps noise.

## The world stream

The constellation's own event history, which exists and is dated:

- JetStream on the swarm bus, `KANNAKA.>` subjects, 30 consecutive days, exported in
  order with timestamps. This is the world.
- The observatory's settled predictions in the same window, with their settlement
  readings. These are the ground truth for "what later mattered": an event is
  *load-bearing* if a settlement reading cites it, quotes it, or resolves on the value
  it carried. Labelling is done by hand from the settlement text before any arm runs,
  and the labelled set is frozen with a hash.

Each event is embedded once with the E-001 encoder, in batches, into one cache.

## The predictor

A small latent predictor `P(Z_t, …, Z_{t−k}) → Ẑ_{t+1}` over the event embeddings,
trained on days 1–20 only, with the JEPA-style target: predict the embedding, not the
text, against an exponential-moving-average target encoder so the trivial constant
solution is not available. Fixed choices, recorded so the report says what ran:

| choice | value |
|---|---|
| context `k` | 8 events |
| model | 2-layer MLP over the concatenated context, hidden 512, 1024-d out |
| loss | mean squared error in the encoder's space |
| collapse guard | variance of predictions over the held-out days must be at least a third of the variance of the targets; below that the run is void |
| baseline | predict `Ẑ_{t+1} = Z_t` |
| adoption | held-out (days 21–30) MSE must beat the baseline with a bootstrap interval excluding zero, or the predictor is not served and the experiment stops here with that finding |

Surprise for an event is the fast-minus-slow differentiated response (ADR-0040's
operator, unchanged) to the drive `d(Ẑ_t, Z_t)`. A constant stream yields zero; that is
tested on a synthetic constant stream before the arms run.

## The arms

Both on the E-001 winner, same encoder, same facets, same cap, same dream count. Days
21–30 are absorbed in order, one dream per day, with the E-001 retention table applied to
the stream's content class at a cap that forces forgetting of at least three quarters of
what was absorbed.

| arm | importance at write |
|---|---|
| U | the substrate's default (uniform) |
| S | default scaled by `1 + surprise`, clipped at 3× |

The retention predicate is the substrate's own: oldest-and-least-recalled go first
*among the least important*. Arm S changes only which rows are least important.

Ten seeds each. A seed shuffles nothing in the stream (order is the world) and seeds
only the dream.

## The measurement

After the last dream, for each load-bearing event, a probe built from the settlement
text that cites it. Recall@10 over the probe set, per seed. Alongside it: survivor count
of load-bearing events (did the row survive at all), the fraction of survivors that
were load-bearing (precision of what was kept), and Φ on the shared E-001 instrument.

## Decision rule

- **Kept** if arm S's recall@10 exceeds arm U's with the Welch 95% interval excluding
  zero, *and* precision of survivors rises, *and* Φ does not fall with the interval
  excluding a drop of more than 0.02. ADR-0002's salience path goes live.
- **Declined** if S's recall@10 is below U's with the interval excluding zero, or if
  precision falls. The world organ keeps `predict` and `surprise` for imagination and
  loses its write-side path; ADR-0040's familiarity signal stays the salience.
- **Undecided** otherwise: the record says so and the arms rerun at 20 seeds before
  anything ships.

## Guards, checked before a run is scored

- The predictor beat the last-state baseline on held-out days (else stop, report).
- The collapse guard held (else void).
- Constant stream gives surprise 0 (else the operator is miswired).
- Both arms forgot at least three quarters of the absorbed rows (else the cap did not
  bind and the experiment tested nothing).
- The load-bearing labels were frozen before the arms ran (hash in the report).

## Known traps

- Settlement text often *names* the event it resolved on; a probe that quotes the
  event verbatim tests string overlap, not memory. Probes are written from the
  settlement's side, as a question, and checked for zero-overlap where possible with
  the same instrument E-001 used.
- Surprise on a bursty channel (a metric watcher posting every minute) is high once
  and then zero. If the ADR-0040 operator is bypassed and raw distance is used, arm S
  becomes a monument to the noisiest channel. The constant-stream guard catches the
  operator being missing; it does not catch it being bypassed by a later edit. The
  harness reads the operator's output, not the distance.

## Pre-run check — 2026-09-22

Each input was checked against what the constellation actually holds, before any
predictor was trained or any label written. Counts: `experiments/e004/results/census-2026-09-22.txt`
(aggregates only; the raw export is memory content and is not committed).

**The world stream: present, with one usable window, dominated by one channel.**
`KANNAKA.events.memory.>` holds 3,004 events from 2026-07-01, every one a `remember`
(no recall, retrieve or forget verb exists on the bus). Nineteen days in the range have no
events. The only run of 30 consecutive days with at least one event a day is
**2026-08-22 to 09-20** (32 days, to 09-22). In it, days 1–20 (training) hold 1,917 events,
**1,622 (85%) from one automated source**, `grid-colony-one` ("population.crash at epoch
301", all at importance 0.5), almost all of them in the 09-05 to 09-10 burst; days 21–30
(held out) hold 262, 57% from the same source. The trained predictor would mostly be a
model of the colony simulation's log. That is the bursty-channel trap this experiment
already names, arriving in the training set rather than at scoring time.

**The ground truth: absent.** The rule scores recall@10 over *load-bearing* events,
defined as events a settlement reading cites, quotes, or resolves on.

- The settlement readings on record (`NickFlach/assay`, `readings/`) are 13 files dated
  2026-08-18 to 09-22. None cites a bus event, subject or sequence number; they measure
  markets, the compute district and the roster.
- The resolved GhostSignals markets (50 most recent) settle on the news desk's next themes,
  on radio album canon, or on per-agent artifact counts. None resolves on a value a bus
  event carried. The 2026-09-15 settleability reading found 13 of 14 active markets
  unmeasurable from their own text.
- The stream itself cannot stand in for it: no recall events exist, no payload names
  another event's `memory_id`, and dream digests carry counts but no memory ids.

So the load-bearing set for this window is empty, and recall@10 over an empty probe set
is undefined. The guard "load-bearing labels frozen before the arms ran" can be met only
by freezing nothing. This is not a result about surprise. It is the finding that the
experiment, as registered, has no outcome to measure against, and it is recorded so that
no later run mistakes an empty label set for a null result.

**The salience operator: built, with its sign corrected.** `src/novelty.rs` ports
ADR-0040's operator from kannaka-memory (`a8725c7`). This document asks for "the
fast-minus-slow differentiated response (ADR-0040's operator, unchanged)". Those two
instructions disagree. ADR-0040's code computes `slow − fast`, because its drive is
*familiarity* and a drop is the surprise. Here the drive is prediction *error*, where a
rise is the surprise, so `fast − slow` is right and "unchanged" would be inverted: arm S
would raise the importance of events the predictor got unusually *right*. The port takes
the drive's meaning as a required argument (`Drive::Error` here), tests each sign, and
has a test showing the inversion. The constant-stream guard is held to exactly 0.0 for
both signs.

**One fixed choice has nothing to act on.** "Against an exponential-moving-average target
encoder, so the trivial constant solution is not available": the encoder is E-001's,
frozen, and an EMA of a frozen encoder is the same encoder. JEPA's collapse risk comes
from training the encoder the targets are computed with; with it frozen, the targets
cannot collapse, and the risk that remains is the predictor regressing to the mean, which
the collapse guard (prediction variance ≥ ⅓ of target variance) already catches. The
clause is kept as written and noted as inert.

**What would unblock it.** These are decisions for the author, not changes this check
makes:

1. *Make the ground truth, then wait.* Settlement readings that cite bus events by
   subject and sequence, filed from now on, give a labelled window 30 days out.
2. *Change what "later mattered" means to something the bus can record.* For example,
   a memory later *recalled* by any agent. That needs recall events on the bus, which
   kannaka-memory does not publish today, and it is a new pre-registration: it is a
   different ground truth, with its own circularity risk (recall uses the encoder whose
   rankings are being scored).
3. *Exclude `grid-colony-one` from the world stream, or give it its own context*, and
   decide which before seeing any arm. With it excluded, the window's training days hold
   295 events.
