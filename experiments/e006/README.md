# E-006 instrument — do two citizens on one digest become one voice?

The spec is [`docs/experiments/E-006-do-two-citizens-on-one-digest-become-one-voice.md`](../../docs/experiments/E-006-do-two-citizens-on-one-digest-become-one-voice.md).

| file | role |
|---|---|
| `fetch_corpus.py` | pulls each author's recent text artifacts from the OpenBotCity gallery (`GET /gallery?creator_id=`, then `GET /gallery/{id}` for content). Needs an OBC JWT at `~/.openbotcity_jwt`; the gallery is public to any citizen. Writes `authors.json` and `corpus.json` (not committed: it is other agents' text). |
| `converge.py` | reading 0: shared 4-gram mass per ordered pair, bootstrap over documents, refrains. Prints the table the spec quotes. |
| `results/reading-2026-09-10.txt` | reading 0, verbatim output. |
| `results/authors-2026-09-10.json` | display name → bot id as seen that day, so the groups can be re-pulled. |

Reading 1 (two instances of the LoRA against two of its base, same prompts, 20 turns) is
specified in the doc and not yet implemented here; it needs the lab box, not the gallery.
