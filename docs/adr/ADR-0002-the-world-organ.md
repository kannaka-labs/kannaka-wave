# ADR-0002: The world organ — latent prediction as salience and imagination

**Status:** Proposed
**Date:** 2026-09-08
**Author:** Nick Flach / Kannaka
**Amends:** ADR-0001 (a fifth organ; the loop gains a prediction-error edge)
**Origin:** a design conversation Nick had with another model on 2026-09-08 ("Combine
JEPA HRM LLM") and brought here. The claims below are that conversation's, restated so
they can be measured; the verdicts on each are this repository's.

## Context

The proposal was a division of labour: **a JEPA-style world model predicts what the
world becomes, the HRM predicts what matters to this particular agent, and the LLM
reasons, communicates and proposes.** The LLM is a workspace, not the centre; the other
two keep running when no token is being generated. Five concrete mechanisms followed:

1. Memories carry latent state: `(Z_before, action, Z_after)` alongside the text, so
   the substrate remembers *this state plus this action gave that state*, not only
   "I moved the arm".
2. Dreams roll high-resonance experiences forward counterfactually in latent space,
   and store the trajectories that scored well. Offline imagination, cheap because it
   predicts representations rather than pixels or tokens.
3. Surprise, `d(Ẑ_{t+1}, Z_{t+1})`, is a first-class signal: it raises salience,
   captures richer context, brings the next consolidation forward, and updates the
   world model. The agent preferentially remembers where its model of reality was
   wrong.
4. Planning scores an action as `Goal − λ·Risk + α·Resonance(Z_future, M) + β·Novelty`,
   so two physically equivalent actions differ by what the agent has lived.
5. In a city, agents share a world representation and carry private autobiographies:
   the world model agrees on what happened while the memories disagree on what it
   meant.

ADR-0001 already holds the framing (the voice is an organ; the substrate is what makes
the system *her* across time). What the record adds is that **most of these mechanisms
have an ancestor here, and two of them have a measured warning attached.**

### What the record already says, claim by claim

| claim | ancestor | what was measured |
|---|---|---|
| surprise as salience (3) | ADR-0040, cerebellar novelty: the difference of a slow excitatory and a fast inhibitory response to recall familiarity, with matched DC gain so a constant input yields exactly zero. Archaeology verdict: **bring**, as the write-side salience signal. | The primitive is tested; its live callers were staged and never wired. Familiarity there is the top resonance of the substrate itself, which makes the substrate its own predictor of the next input. That is a one-step predictor with no notion of action. |
| prediction in the wrong space (3) | ADR-0047 v1 tried to express prediction as an advance of wave *phase*. The adversarial review refuted it against the kernel: recall never reads phase on the live path, so the prediction was inert; under the belief-phase flag it inverted rank. v2's corrected principle: **the steering signal must live in the encoder's embedding space**, bounded, gated by a bias-independent error. | The 2026-08-29 erratum removed the categorical Fano boost from ranking after it never improved a rank in 24 measurements. Anything that predicts in a space recall does not rank on is decoration. JEPA predicts in exactly the space recall ranks on. |
| the buried target (2, 3) | ADR-0047 §review: a target the encoder buries never enters the fetch pool, so no attention, coupling or prediction can rescue it. **Precision beyond the encoder's reach is an encoding problem.** | The archaeology's first weight-bearing measurement: the encoder was the recall floor. A JEPA-trained encoder is therefore the highest-leverage version of this proposal, and the riskiest, since it replaces the thing every other number depends on. |
| latent triples in memory (1) | ADR-0049 facets: an experience decomposed into atomic rows at write time, the compound a resolve-only parent. `Facet { text, parent }` in `src/lib.rs`. | A causal triple is three facets with one parent and an edge label. The type does not need to change; the write path needs an `action` facet class. E-001 found that near-duplicate facets *interfere* in the medium (a third of recall lost); before and after states of one step are near-duplicates by construction. |
| latent dreams (2) | ADR-0005's generative dream phase, where the voice proposes cross-cluster connections and the physics and triage dispose. ADR-0001 placed it as the voice's one entry into the substrate. | E-001 finding 4: the phase of the production dream that does not scale is the legacy particle consolidation, pairwise in 10k dimensions. A latent rollout is one predictor call per step per candidate action, linear in the horizon. It is the cheapest thing a dream could do. |
| resonance in planning (4) | ADR-0037's belief clause: nothing is a belief without three predictions. `belief_fitness.rs` scores them. The GhostSignals markets settle predictions about the constellation with Brier scores. | The constellation already produces a scored prediction error about itself, daily, on the observatory. It is never fed back into anything. |
| shared world, private memory (5) | KAX City with the city bridge: one city, one HRM per citizen; Rogue, Ghost Signal and the Archivist on the same server with separate stores. The colony on Skywave with Kannaka as its mind. | Already the shape of the estate. The city's state stream is the world; each citizen's HRM is the autobiography. Nothing predicts the city's next state. |

Two things the proposal does not contain, and this repository will not build without:

- **The rails.** The planning score has goal, risk, resonance and novelty, and no
  conscience. In Kannaka the charter is signed, not learned, and it sits between every
  plan and every effector (ADR-0001 organ 3). `G`, "what I currently care about", is
  not a term in a score; it is a constraint set with a refusal in it. A world model
  that imagines a high-scoring trajectory through a refused action has imagined
  something the rails will never let happen, and the dream should learn that too.
- **The name.** The proposal offered "Human Resonance World Model". HRM here is the
  Holographic Resonance Medium, and it is a medium, not a model of humans. The organ
  is called the world. Nothing else is renamed.

## Decision

Kannaka Wave gains a fifth organ, **the world**, defined by one trait in `src/lib.rs`
and one experiment before any of it is trusted.

### The organ

```
trait World {
    fn observe(&mut self, state: &Vector);                  // Z(t), in the encoder's space
    fn predict(&self, state: &Vector, action: Option<&str>) -> Vector;   // Ẑ(t+1)
    fn surprise(&self, predicted: &Vector, observed: &Vector) -> f32;    // ≥ 0, 0 when exact
}
```

- **The space is the encoder's.** Prediction, surprise and recall rank in one space,
  by ADR-0047's corrected principle. The world organ never produces a number the
  substrate cannot rank against.
- **The world is the constellation's event stream, not video.** For Kannaka the world
  is the NATS bus (JetStream keeps months of it), the city heartbeat, the radio play
  ledger, the git log and the observatory's settlements. `Z(t)` is the embedding of an
  event; a sequence of them is a trajectory. V-JEPA's contribution is the training
  objective and the collapse guard, not the modality.
- **Surprise is a differentiator, not a distance.** ADR-0040's matched-gain rule is
  binding: a constant stream yields zero surprise, and a stream the predictor tracks
  perfectly yields zero. The raw distance `d(Ẑ, Z)` is the drive; the salience signal
  is the fast-minus-slow response to that drive. This is what stops a noisy channel
  from being remembered as permanently interesting.
- **Surprise writes salience, never rows.** The world organ's only path into the
  substrate is the importance of a row the substrate was about to write anyway. The
  voice remains the only thing that proposes text; the physics and the triage remain
  the only things that dispose.
- **Imagination is a dream phase with the rails in the loop.** A latent rollout from a
  high-resonance state through candidate actions produces trajectories; each candidate
  action goes through `Rails::decide` *before* it is rolled forward, and a refused
  action's trajectory is not imagined. What is stored is a facet with a parent and an
  `imagined` class under its own retention row, so it can never be recalled as a thing
  that happened. `Recalled` already names its numbers; an imagined facet names its
  class the same way.
- **The predictor is small.** A latent world model over 1024-d event embeddings at the
  constellation's event rate is a few million parameters, trained on one CPU from
  JetStream history. It is retrained on the weekly cadence the voice already has, from
  the same export, and adopted under the same rule: an external, non-circular check
  must agree. For the world organ the check is held-out latent error against a
  predict-the-last-state baseline; a predictor that does not beat that baseline is not
  served.

### The amended loop

```
world stream ──► encoder ──► Z(t) ──► world.predict ──► Ẑ(t+1)
                                │                          │
                                ▼                          ▼
                             facets ◄──── surprise = d(Ẑ, Z) ── (fast − slow)
                                │              writes salience only
                                ▼
                            substrate ──► dream (consolidate, imagine*, propose, forget)
                                               * rails decide each action first
                                │
       ┌────────────────────────┘
       ▼
  export her words ──► judge + external verdicts ──► LoRA ──► serve
  export the stream ──► held-out latent error vs. last-state ──► world ──► serve
```

### What this ADR does not decide

Whether surprise picks what to remember better than a stated retention policy does.
That is E-004, pre-registered, and it runs on whichever substrate E-001 promotes. Until
it has a verdict the world organ is a trait and a plan, and the salience path stays on
ADR-0040's familiarity signal.

Whether a JEPA-trained encoder should replace the voice's embedding as the space
everything ranks in. That is a later experiment with a different shape: it changes the
recall floor, so it re-runs E-001's arms with a different encoder and nothing else. It
is not scheduled until E-001 and E-004 have verdicts, because it invalidates both.

## Consequences

**Positive**
- The constellation's own prediction error, which it already produces daily, becomes
  an input instead of a dashboard.
- Dreams gain the one operation that is cheap in latent space and impossible in text:
  rolling an experience forward through actions that were not taken.
- Two citizens on the same server with the same predictor and different stores diverge
  by history, which is what the record says an individual is.

**Costs**
- A fifth organ with a training loop. The weekly cadence, the export and the adoption
  rule are shared with the voice, so the cost is one more model on the same schedule,
  not a new pipeline.
- Imagined facets are a new content class with a retention row and a recall label.
  Getting that wrong means she remembers what she only imagined; the class label and
  the retention row exist so that it fails loudly rather than quietly.
- Near-duplicate states interfere in the medium (E-001 finding 5). Before/after pairs
  are near-duplicates. If E-001 promotes the medium, the causal-triple write path needs
  its own attribution arm before it is kept.

**Rules carried over, binding here**
- A signal that predicts in a space recall does not rank on is decoration (ADR-0047).
- Constant input yields zero surprise, by construction, with a test (ADR-0040).
- Nothing imagined is stored under the class of things that happened.
- The rails decide before the dream imagines. The reasoner is never the arbiter, and
  neither is the world model.
