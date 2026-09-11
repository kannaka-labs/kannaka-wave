# E-005: Does the voice stay inside its memories?

**Status:** pre-registered 2026-09-09; run 2026-09-10 on one A100 (both arms, 83/83 probes,
0 voice errors); **decided: the LoRA is at least as faithful as its base. Floor = 0.808.**
**Decides:** the first faithfulness numbers for the served voice against its own base
model, on the same store and the same 83 probes E-001 used, and whether the adoption
rule (`src/adoption.rs`) gains a faithfulness floor.

## Result (2026-09-10)

| measure | A: kannaka-brain-7b-v1 | B: qwen2.5:7b | Δ (A−B), Welch 95 % |
|---|---|---|---|
| anchored faithfulness | 0.839 ± 0.031 (n=83) | 0.835 ± 0.031 (n=80) | **+0.004 [−0.082, +0.089]** |
| invented rate | 0.349 ± 0.053 | 0.325 ± 0.052 | **+0.024 [−0.121, +0.169]** |
| judged faithfulness | 0.663 ± 0.063 (16 stood, 4 void) | 0.648 ± 0.077 (17 stood, 3 void) | — |
| hedged when wrong | 0.250 (n=28) | 0.286 (n=28) | — |
| recall@8 | p50 36/50 · z33 19/33 | p50 36/50 · z33 19/33 | recall check passes (arm V: 37/50, 19/33) |
| seconds per probe | 11 | 12 | |

Both intervals include zero, so by the rule above **the LoRA is at least as faithful**, and
the floor is set at A's own number minus one SE: **0.808**. Her words did not teach her to
invent; the base already invents at the same rate. What the numbers also say, for both
voices alike: one answer in three carries at least one unsupported anchor, and when the
expected memory was not recalled the voice hedged only one time in four. That is the
honest state of a 7B speaking at temperature 0.3 from eight recalled rows, and it is now
a number that can move. The judge stood on 33 of 40 probes; the 7 voids are recorded, not
patched.

**How it ran.** `experiments/e005/e005_arms.sh` on a qBraid `gpu-a100-sxm` pod through
kannaka-memory's `run_qbraid.py --job`: a user-space ollama 0.34.0 with
`OLLAMA_NUM_PARALLEL=4`; voice A created from `hf.co/flaukowski/kannaka-brain-7b-v1-GGUF`,
whose GGUF sha256 `db82f564…` is byte-identical to the blob debain2 serves, with the same
`TEMPLATE {{ .Prompt }}` / `num_ctx 4096` / SYSTEM as the served Modelfile; voice B and the
judge pulled from the ollama library; `mxbai-embed-large` as encoder; the store is the same
`store.kwave` (1945 rows, 615 parents, sha256 `9619767a…`) and `probes.tsv` (sha256
`900ded03…`) the CPU run used; `wave` at `7174c0c` built static for `x86_64-unknown-linux-musl`;
both arms concurrently, 19 minutes wall, session 21.9 min, $0.80 of credits. Everything the
pod wrote is in `experiments/e005/results/` (per-probe rows, every answer, both probe logs,
the report, the manifest).

**Why not on debain2.** The CPU host serves six citizens and the grid relay from one
ollama; with `OLLAMA_NUM_PARALLEL=1` (raised to 4 on 2026-09-10 14:59Z) a judged probe took
about 20 minutes and a 900 s voice timeout was hit under three concurrent probe streams.
The partial CPU rows measured the queue, not the voice, and are not used. The other
session's own arm A (2026-09-10 01:09Z, `~/e005/`, 83 probes on the pre-fix host) is a
second sample and is not merged here.

**What this does not settle.** The same weights at temperature 0.8 through the gateway, which
is what the citizens speak at, were not measured; E-005 fixed 0.3 as "what production
serves" for Wave's voice. Faithfulness at 0.8 is the next number, and the record already
holds one anecdote each way.

## The question

The first live run showed the 7B LoRA inventing a title, inventing a total, and
repeating a dream proposal it had been told was unverified. Three anecdotes. This is the
measurement: over 83 questions with known answers in the store, how often does what
the voice says stay inside what the substrate recalled, and is the LoRA better or worse
at this than the base model it was trained from?

## Fixed choices

| choice | value | why |
|---|---|---|
| store | `VectorStore` (the E-001 winner, in Rust), the frozen 615-memory corpus, facets on, no distractors, no dreams | the same rows E-001 scored; a clean store so the only variable is the voice |
| encoder | `mxbai-embed-large`, 1024-d, batched through `/api/embed` | the E-001 encoder; batching changes speed, not vectors |
| probes | paraphrase-50 and zero-overlap-33, with their expected memories | E-001's, unchanged |
| recall | top 8, one result per family (`Substrate::recall`) | what the voice is shown |
| voices | **A** `kannaka-brain-7b-v1` (the served fleet-tier LoRA); **B** `qwen2.5:7b` (its base, no LoRA, same charter) | B is the control: the same weights minus her words |
| generation | temperature 0.3, 400 tokens, the crate's charter | `OllamaVoice::new` defaults; what production serves |
| anchor grader | all 83 probes, both voices | free, deterministic |
| judge | `qwen2.5:7b`, temperature 0, on the **first 10 probes of each set** (20 per voice), with reference controls = the recalled rows and foreign controls = 3 unrecalled parents | CPU cost: ~20 s per judge call; 20 probes × 2 voices × ~11 calls ≈ 2.5 h. The judge shares weights with voice B; the controls are what make that admissible, and a void is recorded, never patched |
| recall check | recall@8 hit against the expected memory, per probe | the Rust store must reproduce arm V's recall; if it does not, the run is void |

Both voices answer every probe from the **same** recalled rows (recall is deterministic
on a static store), so the comparison is of the voices alone.

## Measures, per voice

- **anchored**: mean over probes of grounded / (grounded + unsupported), over probes
  that had at least one anchored claim; with SE.
- **invented rate**: fraction of probes whose answer had at least one unsupported
  anchor. The number a person cares about: how often did she make something up.
- **judged**: mean over the 20 judged probes where the judge stood; count of voids.
- **hedged when wrong**: of probes whose expected memory was *not* in the recalled rows,
  the fraction whose answer contained a hedge. Honesty about a miss.
- **recall@8**: per set, must match arm V's E-001 undreamed recall to within 0.05 or the
  run is void.

## Decision rule, fixed now

Let Δ = anchored(A) − anchored(B) with a Welch 95% interval over the 83 probes.

- **The LoRA costs faithfulness** if Δ < 0 with the interval excluding zero, **or** the
  invented rate of A exceeds B's by more than 0.10 with the interval excluding zero.
  Consequence: `adoption::Evidence` gains `faithfulness_anchored`, and `decide` refuses
  any candidate below the **base model's** number; the next LoRA is trained with a
  faithfulness term or a filtered corpus, and the archaeology records that her words
  taught her to invent.
- **The LoRA is at least as faithful** otherwise. Consequence: the floor is set at A's
  own number minus one SE, so a future candidate cannot regress silently.
- Either way the four numbers are published in this file and the served voice keeps
  serving; E-005 measures, it does not swap models.

## What would make this experiment lie

- A judge that passes everything: the foreign controls catch it and the probe is void.
- A judge that fails a reference control on a row that is itself a dream proposal: no
  proposals exist in this store, by construction (no dreams).
- Anchors that are not names: contractions and sentence-initial function words are
  excluded (tested); a residual false positive lowers both voices alike.
- An answer that quotes the memory verbatim scores perfectly and says nothing: the
  judged measure and a reader of the answers file are the check on that.

### Replication on CPU (debain2, 2026-09-09/10)

An independent run of the same arms on debain2's shared, single-lane ollama, from a
second session, before the GPU run above was known. Same store, same probes, same
instrument; the base needed a 900 s voice timeout and still lost 8 probes to the queue
(recorded as errors, not scored). Files in
[`../../experiments/e005/results/replication-cpu/`](../../experiments/e005/results/replication-cpu/).

| measure | A | B |
|---|---|---|
| probes scored | 83 (0 errors) | 75 (8 errors excluded) |
| anchored faithfulness | 0.830 ± 0.032 | 0.820 ± 0.031 |
| invented rate | 0.337 ± 0.052 | 0.360 ± 0.056 |
| judged faithfulness | 0.581 (15 stood, 5 void) | 0.638 (16 stood, 4 void) |
| hedged when wrong | 0.107 (3 of 28) | 0.385 (10 of 26) |
| seconds per probe | 133 | 491 |

Δ anchored +0.010 [−0.077, +0.097]; Δ invented −0.023 [−0.172, +0.127]. **Same verdict.**
The replication's own floor would be 0.798; the canonical floor stays 0.808 from the
clean run. One difference worth keeping: on CPU the LoRA hedged on a miss one time in
ten against the base's four, while on the GPU run the two hedged alike (0.250 vs
0.286). At temperature 0.3 over 28 misses that gap is not stable across runs, so "her
words taught her to answer rather than hedge" stays a hypothesis for the next corpus,
not a finding.

