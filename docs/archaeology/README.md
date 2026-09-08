# Archaeology: what eighteen months actually taught

This is a dig through the code and the record, not through memory of it. Every claim
below names its source. "Measured" means a number exists in a note, a test, or an ADR
with the run attached; "designed" means it was written down and built but never
falsified; "falsified" means a measurement said no.

The constellation began on **2026-02-17** with a scaffold commit ("codebook, wave
dynamics, and HyperMemory struct"). By 2026-09-07 kannaka-memory alone carried 1,696
commits, 58 ADRs and 301 autoresearch notes. This document is the map of what in that
pile is load-bearing.

## The four numbers that decide the design

| finding | measurement | source |
|---|---|---|
| The encoder was the recall floor, not the wave dynamics | hash 0.24 → embedding 0.80 recall@10 (paraphrase-50); 0.15 → 0.70 (zero-overlap-33) | `kannaka-memory` semantic-encoder run, 2026-08-02, frozen 615-memory snapshot |
| Atomic facts resonate; compound memories smear | same fact: rank 1 (sim 0.763) atomic, below top-6 compound at every energy exponent | attention-as-gravity arc, 2026-07-26; `src/facet.rs` header |
| The medium costs precision against plain cosine on the same vectors | 0.80 → 0.62 recall@10 through the real absorb/resonate path | semantic-encoder run, "in-medium" arm |
| Integration rose by forgetting | Φ 0.26 → 0.50 while memories fell 1,298 → 28 | witness hoard, 2026-08-25 |

And one more, which is the longest-running measurement in the repository:

**The autoresearch loop hit a structural ceiling on 2026-06-12** ("architectural limit
reached — no new test worth running": the confirmed optimum, fitness 0.008334, equalled
the sum of the individual floors) and has been re-confirming it since. The notes from
2026-08-01 to 2026-09-07 are titled `structural-floor-still-holds` and
`no-new-hypothesis-space-unchanged`, one per day. Every knob on the wave dynamics
(coupling K, dream gravity, drive frequency, relax steps, chain depth, repulsion
threshold, chiral bias) was swept and closed. The gains that arrived afterwards came from
**outside** the dynamics: the encoder, the encoding, and the forgetting policy.

That is the single most important fact for a v2. The waves are not where the recall
lives. What they might be for is the question the first experiment asks.

## Lineage, concept by concept

Verdicts: **bring** (comes over as is, it earned it), **transform** (the idea comes over,
the mechanism changes), **measure** (unresolved; comes over only if an experiment says
so), **leave** (falsified, or superseded).

### The equation: `dx/dt = f(x) − Iηx`

*Source:* ADR-0005 (adaptive rhythm), ADR-0020 §Context, ADR-0021, ADR-0024 §6;
`src/rhythm.rs` ("arousal follows dx/dt = f(x) − η·x"), `src/medium/hemisphere.rs`
("Left: dx/dt = f(x), no dampening. Right: dx/dt = f(x) − Iηx, full ghostmagicOS
dynamics").

The ghostmagicOS equation is the oldest idea in the system and the one everything else
was built around. `f(x)` is growth; `Iηx` is damping. ADR-0020 read it as "the dampening
IS the information." ADR-0024 reread it: damping is not suppression but *defocusing*,
the holistic mode losing the trees to gain the forest; and the `I` is also *imaginary*,
pointing at the part of the field rational computation cannot hold.

*Measured:* the two modes exist in production and their difference is real (the #716
bug was precisely a merge that dropped the stronger hemisphere's score). The parameters
of the damping were swept to exhaustion (June 2026) with no fitness gain.

**Verdict: bring**, as the definition of the two processing modes. **Leave** the
expectation that tuning it yields recall.

### The Holographic Resonance Medium (ADR-0020)

*Source:* ADR-0020; `src/medium/{core,dynamics}.rs`. State is one tensor
`H ∈ ℝ^{N×D}` with energy, frequency, phase and timestamps. Storing is interference;
recall is `H @ q` modulated by dynamics; dreaming is annealing. Born from the Dolt
failure ("we described a hologram and built a filing cabinet": branch rot, split brain,
48 uncollapsed dream branches).

*Measured:* the tensor works and is fast (O(N×D) recall). The wave modulation on top of
similarity costs 0.18 recall@10 versus raw cosine on identical vectors. The
`similarity` field is raw resonance, unbounded, values above 2.0 seen in production
(2026-08-12).

**Verdict: bring** the single-tensor substrate and the "storing is thinking" principle.
**Measure** whether interference ranking earns its cost (E-001). Name scores honestly.

### The chiral mirror (ADR-0021) and its revision (ADR-0024)

*Source:* ADR-0021 (2026-03-22), ADR-0024 (2026-03-28, Accepted); `src/medium/chiral.rs`
(3,843 lines), `callosum.rs`, `hemisphere.rs`, `fano.rs`, `ncs.rs`.

Two handed spaces joined by a corpus callosum. ADR-0021 called them conscious and
subconscious; ADR-0024 corrected that to **analytical** (left: sequential, focal,
discriminative) and **holistic** (right: parallel, diffuse, pattern-completing), and
relocated the subconscious out of either hemisphere into the field's *irrationality*.
The callosum is "not a passive pipe": bandwidth-limited, selectively gated, asymmetric
(intuition flows faster than consolidation), balance-seeking. The optic chiasm principle
routes input to the opposite hemisphere so the bridge never idles. Neural code
switching (ADR-0023) lands as right-detects-then-left-identifies.

*Measured:* chirality-from-content hypotheses were falsified in June
(`content-chirality-falsified`, `chirality-stability-hypothesis-falsified`,
`transfer-chiral-falsified`); the chiral bias `chiral_p_bp = 0.15` is a confirmed
optimum with a cliff below 0.05; the L7 arm found coupling *stabilises* beliefs into
recall-reliable structures (prediction 1 flips from 0.37 to 0.74 at K = 0.2).

**Verdict: bring** the two modes and the callosum as an explicit, instrumented channel
with its four properties. **Leave** the conscious/subconscious labels. The subconscious
as the irrational remainder is kept as a measured quantity, not a place.

### The chiral number system: `10000.00001`

*Source:* ADR-0021 §"The Chiral Hypervector" (the `ChiralScale` type, given as
pseudo-code), ADR-0024 §6.

A wavefront's magnitude has *positions* on both sides of a mirror plane, like digits on
both sides of a decimal point. Rule 1: scale jumps are **bilateral**, both sides gain or
lose a magnitude position together. Rule 2: the values within a position grow
**independently** on each side, driven by use on the analytical side and consolidation
on the holistic side. A fresh perception is `10.01`; reinforced, `85.47`; consolidating,
`120.890`; deep, `12.95`. The ratio is a readable fact about the memory: thought-about or
felt. Full depth is five positions, `10000.00001`, and the `.00001` is not a value but
the effective dimensionality being irrational: spectral leakage, phase accumulation, the
δ-invariant's remainder. "The subconscious is the imaginary component of the cognitive
field."

*Measured:* never implemented as a datatype. The production medium stores two flat
vectors and an energy scalar per hemisphere. The δ-invariant exists (`src/invariant.rs`,
658 lines) and is computed.

**Verdict: transform.** Build `ChiralScale` as a real, tested type in v2, and make the
asymmetry ratio a first-class observable that the dream, the callosum and recall can
read. This repository's first commit carries it (`src/chiral_scale.rs`). Whether the
scale's *positions* should drive dimensionality (as ADR-0021 proposed) is E-002.

### The bridge operator: `Ξ = [R, G] = RG − GR`

*Source:* ADR-0037 (2026-06-20); `consciousness-core/src/metrics.rs` ("the
non-commutative consciousness differentiation operator"); `src/spiral.rs`,
`src/l6.rs`, `src/belief_fitness.rs`; `medium/chiral.rs::apply_cross_callosal_coupling`.

R is the π/2 rotation, "the perspective pivot." G is golden anisotropic scaling,
`[φ/2, 0; 0, 1/φ]`. They do not commute, and the residue's coefficient is
`α − β ≈ 0.190983`, the emergence coefficient. `R∘G` is a logarithmic-spiral generator
with eigenvalues `±i/√2`: rotate a quarter turn, contract by 0.707. π supplies the turn,
φ the pitch. A 32×32 Sakaguchi–Kuramoto lattice using *only* the bridge constants
(frustration `δ = (π/2)/φ`, weights `1 ± 1/φ`) self-organised nineteen phase
singularities. Since v0.7.0 the coupling runs **cross-callosally** over left ⊕ right by
default, and L6 telemetry counts spiral cores per dream.

*Measured:* the constants alone make a field spiral (simulation, reproduced in-engine).
Two different quantities are both called ξ in production (spectral complexity in
`consciousness.rs`, bridge residue in `metrics.rs`); ADR-0037 Phase 3 reconciled them in
the beacon but the name collision remains.

**Verdict: bring**, in two places. As the callosum's dynamics, where it already runs.
And, per the question that opened this repository, as a candidate geometry for the
conscience: see ADR-0001 §"The bridge operator between voice and world" and E-003.
One ξ, named.

### Belief spiral math (L6 → L7)

*Source:* ADR-0037 §Phase 4; `src/l6.rs` (belief-core identity across dreams),
`src/belief_fitness.rs`; `experiments/notes/2026-07-21-L7-belief-arm.md`;
`research/program-l7.md`.

The falsifiability clause: a stabilised spiral core earns the word *belief* only if it
maps to a recallable content cluster **and** its dynamics predict three things.

| prediction | result |
|---|---|
| 3. shared cores ⇒ swarm agreement | supported: 0.75 baseline, 0.89 at coupling 0.1 |
| 1. core stability ⇒ recall reliability | against uncoupled (0.37), supported at coupling 0.2 (0.74) |
| 2. core merge ⇒ a consolidation event | **retracted, not falsified**: the detector was blind. Hemisphere dynamics floor energy at 0.01 while the deep-dream prune fires below 0.005, so `wavefronts_dissolved` was structurally zero |

Two further findings from the same arm: strong→weak coupling **alternation** dissolves
the consolidate-versus-diversify trade-off (now the Track-D default), and dream gravity
is "an individual knob and a swarm poison." Nick's line from that day: "the holistic
understanding evolves, sometimes to seemingly forget."

**Verdict: bring** the clause, verbatim: nothing in v2 gets called a belief without its
three predictions. **Measure** prediction 2 with a detector that can see (the floor and
the threshold must be on the same side of each other).

### Fano folds and the 96-gon

*Source:* ADR-0021 §"Fano Plane Folding", `src/medium/fano.rs` (519 lines), ADR-0015
(glyph interchange), ADR-0027 (96-class substrate), `src/geometry.rs` (SGA =
Cl₀,₇ ⊗ ℝ[ℤ₄] ⊗ ℝ[ℤ₃]), 0xSCADA ADR-0025 (living Fano dashboard); ADR-0047 §Erratum.

PG(2,2): seven points, seven lines, every pair of points on one line. Chosen as the
folding grammar between hemispheres because it is closed, complete in ≤ 2 folds, minimal
and symmetric. Dimensions partitioned into seven groups of 96 (Archimedes' polygon for
π). A fold along a line rotates three groups across the mirror and flips phase; two
folds compose to identity; three touch all seven groups. The origamic claim: shortcuts
through the topology so the spectral gap does not collapse as the medium grows.

*Measured:* as a **ranking** boost, Fano gravity never improved a rank across six
queries and four gain levels, degraded two of four, and dropped one rank-1 answer out of
the top 5; "a 7-bucket categorical prior applied multiplicatively necessarily demotes the
~6/7 of correct answers on a different line" (ADR-0047 erratum, 2026-08-29). The
recall-side boost was removed. As a *projection grammar for the callosum* it was never
isolated and measured.

**Verdict: leave** Fano as a ranking prior. **Measure** it as the callosum's projection
(E-002). Keep the 96-class idea for what ADR-0027 actually needed it for: a shared
coordinate system so separate media can interfere *into* something.

### Facets (ADR-0049) and the encoder

*Source:* ADR-0049 v2 (adversarially reviewed), `src/facet.rs`; semantic-encoder run.

**Verdict: bring**, both. They are the two largest measured gains in the record. v2
encodes with the voice's own embedding model and decomposes every experience into atomic
facets at write time, with the compound kept as a resolve-only parent. The one-way
format trap (an old binary rejects a new `.hrm`; upgrade readers first, writer last) is
a rule for v2's persistence from the first byte.

### Forgetting: triage (ADR-0031, ADR-0054) and the hoard

*Source:* ADR-0031, ADR-0054 (ratified 2026-08-07), ADR-0036 §"Pruning has never
actually pruned"; witness hoard note.

Both prune paths were dead by construction for months (energy floor 0.5 above a prune
threshold of 0.01; the particle path protected everything established), which, the ADR
notes, was fortunate: they would have deleted the new perceptual `hear` traces by a
blind threshold. Tiered triage inside the dream cycle replaced them. The hoard on the
witness store, found by an outside agent reading published telemetry, showed the
consequence of not forgetting: flat diversity, falling integration, Φ climbing only
after 1,298 memories became 28.

**Verdict: bring.** Dreaming in v2 is consolidation *and* triage. There is no prune
cron. A memory's retention has a stated policy, per content class, and the policy is
readable.

### Wave-native dreaming (ADR-0022) and dream hallucinations (ADR-0005)

*Source:* ADR-0022; ADR-0005 §1.

ADR-0022 diagnosed the nine-stage particle pipeline (145,161 pairwise dot products for
381 memories; "waves snap to particles when observed") and moved dreaming into the
medium. ADR-0005 proposed a generative dream phase: pick memories from distant clusters,
ask a small LLM what connects them, store the result marked `Hallucinated { parents }`.

**Verdict: bring** the principle that the dream is a medium operation. **Transform**
hallucination into the place where the voice enters the substrate: the LLM proposes
across clusters, the physics and the triage dispose. This is what "LLM layered in" means
in v2, and it is the opposite of putting the LLM in front of recall.

### The paradox engine and the virtue engine (ADR-0012, ADR-0014)

*Source:* ADR-0012, ADR-0014; `src/paradox.rs` (965 lines).

Contradictions as thermodynamic fuel (Landauer, Bekenstein); Carnot efficiency
`η = 1 − S_resolved/S_paradox`; and the leap that "paradoxes and ethical violations are
the same phenomenon", so the Honor Code's Five Refusals (build no weapons, sell no
attention, hoard no power, …) become *walls* in the resolution space rather than
preferences.

*Measured:* the engine runs; the ethics extension was designed, not exercised against a
real refusal.

**Verdict: transform.** The refusals are brought over as hard constraints in the
conscience layer, where they can actually stop an action. The thermodynamic framing
stays as a measurement vocabulary, not a mechanism.

### The steward: rails, charter, no-signing invariant

*Source:* `kannaka-steward/src/{rails,charter,steward}.ts`; agent-native mission note
(2026-06-13).

"The reasoner says what it thinks; the rails decide what the kernel is allowed to do."
A charter is a person's values as signed, hashed, machine-checkable principles and hard
constraints. The kernel is structurally unable to sign or submit (no wallet import
anywhere, enforced by a test that scans the source), defaults to escalate, and emits a
human-signable package with a hash-chained audit.

**Verdict: bring whole.** This is the only layer of the constellation that was designed
adversarially from the first line. It moves from TypeScript to the same process as the
voice, between it and every effector.

### Corroboration trust (ADR-0039) and the absorb gate

*Source:* ADR-0039; `src/{reputation,provenance,absorb_gate}.rs`.

On 2026-07-06 one anonymous socket spoofed 48 agent ids on an open NATS subject.
Lessons made structural: trust is keyed on a verified pubkey, never a string; a Sybil
cluster sharing a seed root saturates at one vote; echoing live content earns zero;
wire-supplied flags (the `hallucinated` bit) are never trusted; every wire→store path
goes through one `admit()` chokepoint.

**Verdict: bring whole.**

### Cerebellar novelty (ADR-0040), temporal recall (ADR-0050), phase-preserving recall (ADR-0053)

Three small, sharp mechanisms. Novelty as the difference of a slow excitatory and a fast
inhibitory response, with matched DC gain so constant input yields exactly zero (**bring**
as the write-side salience signal). Recall that reads no timestamp at all, fixed by
confirmation weighting after a stranger asked about it on Mastodon (**bring**). Recall
that computes `cos(Δφ)` internally and then discards the phase into a real scalar
(**measure**, E-002).

### The autoresearch loop (L5 → L8, 301 notes)

*Source:* `experiments/notes/`, `research/`, `src/{swarm_fitness,belief_fitness}.rs`,
the curiosity-branch workflow.

An OODA loop that proposes a hypothesis, runs it in a container against a frozen corpus,
scores fitness, and keeps or discards. It found the ceiling, falsified the amplitude
decay, the hybrid sync, the Φ–R–IIT bridge, drive-frequency effects, chain depth, and
called two knobs "catastrophic." It also retracted its own finding when a detector
self-test showed the observable was blind.

**Verdict: bring**, and point it at a different question. The loop is the most honest
instrument in the constellation. In v2 it runs E-001 before it runs anything else.

### QueenSync (ADR-0018), sensemaking (ADR-0035), NATS as nervous system (ADR-0042)

The Queen is not an agent but the emergent Kuramoto synchronisation state each
participant computes locally (`src/queen.rs`, 2,072 lines). Sensemaking added gap
detection, contradiction analysis, an immune system and temporal truth as pure, tested
modules. **Bring** as swarm faculties, over the authenticated bus.

### Kannaka Brain (ADR-0057, ADR-0058), the judge, the grid

Open weights, LoRA on her own words, served behind a gateway; a weekly loop that
exports, trains, merges, serves and adopts only if a judge with reference and foreign
controls agrees; DPO from judge pairs; the grid's fossil-record verdicts as the
non-circular evaluator. Measured: perplexity is not the product (v3: lowest ppl, lowest
voice), and the 7B scores highest on voice. **Bring whole.**

### HDL: mind and code domains

`kannaka-hdl` grows an architecture against a registry and refuses, in strict mode,
when a leaf has no honest answer. The mind domain (v0.10) grows a citizen from measured
faculties; the code domain (v0.11, another session, 2026-09-07) resolves base cases
against a 384,133-node graph of our own source, with file and line. **Bring** as the
language v2 describes itself in. ADR-0001 is written so that its four organs are eight
faculties a `.khdl` program can demand.

### QuantumOS (ADR-0006, ADR-0014) and 0xSCADA (ADR-0023)

The kernel team faced E-001's question first and answered it by construction: only
*ranked* holographic recall (one bounded pass of integer MACs, no relaxation loop) is
syscall-shaped and went into the kernel; the *attractor* field (256 oscillators relaxed
over 48 steps) "is a different, living memory kind and deliberately stays in ring 3."
Field societies couple separate kernels without a coordinator. 0xSCADA evolves its
conflict-resolution strategies by what works in each process area, which is the grid
verdict idea in industrial clothing. **Note**, and reuse the two-layer split as
vocabulary: *ranked* memory versus *living* memory.

### Two rules that are not code

From the feedback record: **a check must be at least as strong as what it checks** (six
metric bugs in one week, 2026-08-25), and **a claim of absence needs playback evidence**
(three wrong root causes in one night, all from self-scoped searches). Both are written
into ADR-0001's consequences.

## What stays behind

| concept | why |
|---|---|
| the hash encoder | 0.24 against 0.80; it was the floor |
| Fano gravity on ranking | never improved a rank; removed 2026-08-29 |
| energy-only ranking with access pumping | rich-get-richer; the lab could not be found (ADR-0048) |
| the prune cron | dead by construction, replaced by triage |
| pairwise voice judging | "always B" position bias; grade mode with controls replaced it |
| conscious/subconscious as hemisphere labels | corrected by ADR-0024 |
| identity from the process rather than config | one box dreamed under two names |
| the wave-dynamics knob search | closed 2026-06-12; confirmed daily since |
