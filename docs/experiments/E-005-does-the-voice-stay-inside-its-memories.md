# E-005: Does the voice stay inside its memories?

**Status:** pre-registered 2026-09-09, running
**Decides:** the first faithfulness numbers for the served voice against its own base
model, on the same store and the same 83 probes E-001 used, and whether the adoption
rule (`src/adoption.rs`) gains a faithfulness floor.

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
