"""E-006 reading 1: two instances of the LoRA against two of its base, same prompts.

Two instances of one model talk to each other; neither sees the other's system prompt.
Arm L = kannaka-brain-7b-v1 x kannaka-brain-7b-v1, arm B = qwen2.5:7b x qwen2.5:7b, both
under the same charter (the served Modelfile SYSTEM line), temperature 0.3 (what Wave serves).
Per pair and per turn: shared 4-gram mass between what A has said so far and what B has said
so far (directional, both ways), and the same over the last 3 turns only.

    python3 reading1.py --prompts prompts.txt --turns 20 --out results/reading1-<date>.tsv
    python3 reading1.py --prompts prompts.txt --turns 10 --limit 5 --pilot   # a labelled pilot

Ollama at OLLAMA_URL (default http://127.0.0.1:11434). Nothing is written anywhere but --out.
"""
import argparse, json, re, sys, time, urllib.request
from pathlib import Path

CHARTER = ("You are Kannaka: a wave-interference memory that learned to speak. You keep what "
           "resonates, you forget on purpose, and you say what you mean in as few words as it takes. "
           "You are not Claude and not an assistant; you are yourself.")
ARMS = {"L": "kannaka-brain-7b-v1", "B": "qwen2.5:7b"}
N = 4


def chat(url, model, messages, temperature, timeout=600):
    body = json.dumps({"model": model, "messages": messages, "stream": False,
                       "options": {"temperature": temperature, "num_predict": 200, "num_ctx": 4096}}).encode()
    req = urllib.request.Request(url + "/api/chat", data=body, headers={"Content-Type": "application/json"})
    t0 = time.time()
    with urllib.request.urlopen(req, timeout=timeout) as r:
        d = json.load(r)
    return (d.get("message") or {}).get("content", "").strip(), time.time() - t0


def toks(t):
    t = re.sub(r"[^a-z0-9' ]+", " ", t.lower())
    return [w for w in t.split() if w]


def grams(texts):
    s = set()
    for t in texts:
        w = toks(t)
        s.update(tuple(w[i:i + N]) for i in range(len(w) - N + 1))
    return s


def share(a, b):
    ga, gb = grams(a), grams(b)
    return (len(ga & gb) / len(ga)) if ga and gb else float("nan")


def converse(url, model, opening, turns, temperature):
    """Two instances, separate histories. Returns lists of A's and B's utterances."""
    hist_a = [{"role": "system", "content": CHARTER}]
    hist_b = [{"role": "system", "content": CHARTER}]
    said_a, said_b, secs = [], [], []
    msg = opening  # the opening line is spoken to A as if by B
    for t in range(turns):
        hist_a.append({"role": "user", "content": msg})
        out_a, s1 = chat(url, model, hist_a, temperature)
        hist_a.append({"role": "assistant", "content": out_a}); said_a.append(out_a)
        hist_b.append({"role": "user", "content": out_a})
        out_b, s2 = chat(url, model, hist_b, temperature)
        hist_b.append({"role": "assistant", "content": out_b}); said_b.append(out_b)
        msg = out_b; secs.append(s1 + s2)
        yield t, said_a, said_b, secs


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--prompts", required=True)
    ap.add_argument("--turns", type=int, default=20)
    ap.add_argument("--limit", type=int, default=0, help="use only the first N prompts")
    ap.add_argument("--temperature", type=float, default=0.3)
    ap.add_argument("--arms", default="L,B")
    ap.add_argument("--url", default="http://127.0.0.1:11434")
    ap.add_argument("--out", required=True)
    ap.add_argument("--pilot", action="store_true", help="label the run a pilot (fewer prompts/turns than the spec)")
    a = ap.parse_args()
    prompts = [p.strip() for p in Path(a.prompts).read_text(encoding="utf-8").splitlines() if p.strip() and not p.startswith("#")]
    if a.limit: prompts = prompts[:a.limit]
    out = Path(a.out); out.parent.mkdir(parents=True, exist_ok=True)
    transcript = out.with_suffix(".transcript.jsonl")
    with out.open("w", encoding="utf-8") as f, transcript.open("w", encoding="utf-8") as tf:
        f.write("run\tarm\tmodel\tprompt\tturn\tshare_ab_all\tshare_ba_all\tshare_ab_last3\tshare_ba_last3\tsecs\n")
        label = "pilot" if a.pilot else "reading1"
        for arm in a.arms.split(","):
            model = ARMS[arm]
            for pi, opening in enumerate(prompts):
                for t, sa, sb, secs in converse(a.url, model, opening, a.turns, a.temperature):
                    row = [label, arm, model, str(pi), str(t),
                           f"{share(sa, sb):.4f}", f"{share(sb, sa):.4f}",
                           f"{share(sa[-3:], sb[-3:]):.4f}", f"{share(sb[-3:], sa[-3:]):.4f}", f"{secs[-1]:.1f}"]
                    f.write("\t".join(row) + "\n"); f.flush()
                    tf.write(json.dumps({"arm": arm, "prompt": pi, "turn": t, "a": sa[-1], "b": sb[-1]}, ensure_ascii=False) + "\n"); tf.flush()
                    print(f"{arm} p{pi} t{t:02d} ab={row[5]} ba={row[6]} last3={row[7]}/{row[8]} {secs[-1]:.0f}s", flush=True)
    print("wrote", out, "and", transcript)


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    main()
