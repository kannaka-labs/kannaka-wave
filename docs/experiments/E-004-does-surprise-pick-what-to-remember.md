# E-004: Does surprise pick what to remember?

**Status:** pre-registered, blocked on E-001 (runs on the substrate E-001 promotes)
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
