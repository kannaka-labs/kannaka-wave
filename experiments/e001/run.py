#!/usr/bin/env python3
"""E-001 orchestrator: ten seeds, two arms, one Φ instrument, one results file.

    python run.py --work <dir> --prepare                 embed every text once, in batches
    python run.py --work <dir> --seeds 1..10 [--cycles 30] [--churn 40] [--arms WV]

Per seed:
  arm W  e001-harness wave-build  → kannaka observe --json (fresh process, Φ_hrm before)
         e001-harness wave-run    → kannaka observe --json (fresh process, Φ_hrm after)
         survivors → embed cache → e001-harness phi                    (Φ_e001 after)
  arm V  run_arm_v (in-process)   → survivors → same cache → same phi  (Φ_e001 after)
  both   recall@10 on paraphrase-50 and zero-overlap-33 from the arm's top-10 ids,
         with the zero-overlap invariant re-verified against the FULL corpus text.

Recall knobs are PRODUCTION DEFAULTS (nothing set). The 2026-08-02 encoder run
and the Harbor evals set KANNAKA_RECALL_ENERGY_EXP=0.0 on an undreamed store;
on a dreamed store that knob lets dream-created summary rows outrank everything
at similarity 1.0 (seen on the smoke store), so arm W's 0.62 on record is not
directly comparable with arm W here, and the report says so.

Every number goes to <work>/results.tsv as one row per (seed, arm); every
per-probe rank goes to <work>/runs/<arm>-<seed>.json. report.py reads the TSV.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

# Windows consoles default to cp1252; the report prints Δ and ±. Say utf-8.
for _s in (sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

sys.path.insert(0, str(Path(__file__).resolve().parent))
from arm_v import run_arm_v  # noqa: E402

# Windows Python decodes child output as cp1252 by default; the facet and
# recall JSON carry UTF-8, and a stray 0x8d kills the reader thread and hands
# back stdout=None. Every subprocess call here says utf-8 explicitly.
KANNAKA = Path.home() / ".local" / "bin" / "kannaka.exe"
HARNESS = Path(os.environ.get("E001_HARNESS", r"C:\Users\nickf\Source\kannaka-memory\target\release\e001-harness.exe"))
OLLAMA = os.environ.get("E001_OLLAMA", "http://localhost:11434")
MODEL = "mxbai-embed-large"
DIM = 1024
TOKEN = re.compile(r"[a-z0-9]{4,}")

# The one retention table both arms enforce. Corpus rows carry no prefix and are
# never eligible; distractors are capped so the policy has something to forget.
RETENTION = {"distractor:": {"cap": 60}}

SCRUB = ["KANNAKA_GLYPH_GRAVITY", "KANNAKA_RECALL_TEMPORAL_EXP", "KANNAKA_RECALL_ENERGY_EXP",
         "KANNAKA_TRIAGE", "KANNAKA_FACET_DECOMPOSE", "KANNAKA_SPIRAL_DREAM", "KANNAKA_ENCODER",
         "KANNAKA_ENCODER_MODEL", "KANNAKA_ENCODER_DIM", "KANNAKA_ENCODER_URL", "KANNAKA_DATA_DIR",
         "E001_EMBED_CACHE"]


def log(m: str) -> None:
    print(f"[e001 {time.strftime('%H:%M:%S')}] {m}", file=sys.stderr, flush=True)


# ---------------------------------------------------------------- embeddings (one cache, both arms, the harness too)
class Embedder:
    """Keyed by the exact text, because the harness's CachedEncoder looks up the
    exact string the store hands its encoder."""

    def __init__(self, cache_path: Path):
        self.path = cache_path
        self.cache: dict[str, list[float]] = {}
        if cache_path.exists():
            self.cache = json.loads(cache_path.read_text(encoding="utf-8"))
        self.dirty = 0

    def __call__(self, text: str) -> list[float]:
        v = self.cache.get(text)
        if v is None:
            v = self.batch([text])[0]
        return v

    def batch(self, texts: list[str]) -> list[list[float]]:
        missing = list(dict.fromkeys(t for t in texts if t not in self.cache))
        B = 64
        for i in range(0, len(missing), B):
            chunk = missing[i:i + B]
            req = urllib.request.Request(f"{OLLAMA}/api/embed",
                                         data=json.dumps({"model": MODEL, "input": chunk}).encode(),
                                         headers={"Content-Type": "application/json"})
            with urllib.request.urlopen(req, timeout=1800) as r:
                embs = json.load(r)["embeddings"]
            for t, e in zip(chunk, embs):
                n = sum(x * x for x in e) ** 0.5
                self.cache[t] = [x / n for x in e] if n else e
                self.dirty += 1
            if self.dirty >= 500:
                self.flush()
            if missing and (i // B) % 10 == 0:
                log(f"  embedded {min(i + B, len(missing))}/{len(missing)}")
        self.flush()
        return [self.cache[t] for t in texts]

    def flush(self) -> None:
        if self.dirty:
            tmp = self.path.with_suffix(".tmp")
            tmp.write_text(json.dumps(self.cache), encoding="utf-8")
            tmp.replace(self.path)
            self.dirty = 0


# ---------------------------------------------------------------- scoring
def score(results: dict, expected: dict) -> dict:
    n = len(expected)
    rec, hits, ranks = 0.0, 0, {}
    for pid, rel in expected.items():
        ids = results.get(pid, [])
        rel = set(rel)
        rec += sum(1 for x in ids if x in rel) / len(rel)
        rank = next((i + 1 for i, x in enumerate(ids) if x in rel), 0)
        if rank:
            hits += 1
            ranks[pid] = rank
    return {"recall_at_10": rec / n, "hits": hits, "n": n, "ranks": ranks}


def check_zero_overlap(probes: list[dict], targets: dict) -> None:
    """The premise of the harder set: no ≥4-char token shared with the target's
    FULL content. A broken invariant is an infrastructure error, never a score."""
    for p in probes:
        q = set(TOKEN.findall(p["query"].lower()))
        for c in targets[p["id"]]["contents"]:
            shared = q & set(TOKEN.findall(c.lower()))
            if shared:
                raise SystemExit(f"INFRA: zero-overlap invariant broken for {p['id']}: {sorted(shared)}")


# ---------------------------------------------------------------- Φ (one instrument)
def phi_e001(emb: Embedder, survivors: list[dict], work: Path, tag: str) -> dict:
    vecs = emb.batch([s["content"] for s in survivors])
    f = work / f"phi-{tag}.json"
    f.write_text(json.dumps({"vectors": vecs}))
    out = subprocess.run([str(HARNESS), "phi", "--vectors", str(f), "--k", "8", "--parts", "8"], capture_output=True, text=True, encoding="utf-8", errors="replace", check=True)
    f.unlink(missing_ok=True)
    return json.loads(out.stdout)


def observe_phi(data_dir: Path, env: dict) -> dict:
    """Arm W's native Φ, from a FRESH process, never from the dream's own report."""
    out = subprocess.run([str(KANNAKA), "observe", "--json"], capture_output=True, text=True, encoding="utf-8", errors="replace", env=env, cwd=str(data_dir))
    if out.returncode != 0:
        raise SystemExit(f"observe failed: {out.stderr[-800:]}")
    c = json.loads(out.stdout).get("consciousness", {})
    return {k: c.get(k) for k in ("phi", "xi", "total_memories", "active_memories", "num_clusters", "level")}


# ---------------------------------------------------------------- arm W
def wave_env(dd: Path, cache: Path) -> dict:
    env = {k: v for k, v in os.environ.items() if k not in SCRUB}
    env.update({"KANNAKA_DATA_DIR": str(dd), "KANNAKA_FACET_DECOMPOSE": "1", "KANNAKA_TRIAGE": "1",
                "KANNAKA_ENCODER": "ollama", "KANNAKA_ENCODER_MODEL": MODEL, "KANNAKA_ENCODER_DIM": str(DIM),
                "KANNAKA_ENCODER_URL": OLLAMA, "E001_EMBED_CACHE": str(cache)})
    return env


def run_arm_w(seed: int, work: Path, corpus_p: Path, pool_p: Path, probes_p: Path, cycles: int, churn: int) -> dict:
    dd = work / "runs" / f"w-{seed}"
    if dd.exists():
        shutil.rmtree(dd)
    dd.mkdir(parents=True)
    (dd / "config.toml").write_text(
        '[agent]\nid = "e001-w"\ndisplay_name = "e001-w"\nkind = "agent"\n\n'
        '[swarm]\nenabled = false\nnats_url = ""\nrole = "worker"\n\n'
        f'[encoder]\nkind = "ollama"\nbase_url = "{OLLAMA}"\nmodel = "{MODEL}"\ndim = {DIM}\n\n'
        '[retention]\n' + "".join(f'"{k}" = {{ cap = {v["cap"]} }}\n' for k, v in RETENTION.items()),
        encoding="utf-8")
    env = wave_env(dd, work / "embeddings.json")
    t0 = time.time()
    b = subprocess.run([str(HARNESS), "wave-build", "--corpus", str(corpus_p)], capture_output=True, text=True, encoding="utf-8", errors="replace", env=env)
    if b.returncode != 0:
        raise SystemExit(f"wave-build failed:\n{b.stderr[-1500:]}")
    build = json.loads(b.stdout.strip().splitlines()[-1])
    phi_before = observe_phi(dd, env)
    log(f"W{seed} built: {build} Φ_hrm(before)={phi_before['phi']}")
    out_json = dd / "run.json"
    retention_arg = ",".join(f"{k}={v['cap']}" for k, v in RETENTION.items())
    r = subprocess.run([str(HARNESS), "wave-run", "--corpus", str(corpus_p), "--distractors", str(pool_p),
                        "--probes", str(probes_p), "--cycles", str(cycles), "--churn", str(churn),
                        "--seed", str(seed), "--retention", retention_arg, "--out", str(out_json)],
                       capture_output=True, text=True, encoding="utf-8", errors="replace", env=env)
    if r.returncode != 0:
        raise SystemExit(f"wave-run failed:\n{r.stderr[-2000:]}")
    misses = [l for l in r.stderr.splitlines() if "cache miss" in l]
    run = json.loads(out_json.read_text(encoding="utf-8"))
    run["phi_hrm_before"], run["phi_hrm_after"] = phi_before, observe_phi(dd, env)
    run["build"] = build
    run["wall_s"] = round(time.time() - t0, 1)
    run["dream_ms_mean"] = round(sum(c["dream_ms"] for c in run["cycle_rows"]) / max(1, len(run["cycle_rows"])))
    run["embed_cache_miss_lines"] = misses[-1:]
    (dd / "kannaka.hrm").unlink(missing_ok=True)  # keep the numbers, drop the medium
    return run


# ---------------------------------------------------------------- main
def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--work", required=True)
    ap.add_argument("--seeds", default="1..10")
    ap.add_argument("--cycles", type=int, default=30)
    ap.add_argument("--churn", type=int, default=40)
    ap.add_argument("--arms", default="WV")
    ap.add_argument("--prepare", action="store_true", help="embed every text once, in batches, then exit")
    a = ap.parse_args()
    work = Path(a.work)
    (work / "runs").mkdir(parents=True, exist_ok=True)

    corpus = json.loads((work / "corpus-slim.json").read_text(encoding="utf-8"))
    pool_all = json.loads((work / "local-slim.json").read_text(encoding="utf-8"))
    corpus_texts = {r["content"] for r in corpus}
    pool = [r for r in pool_all if r.get("content") and r["content"] not in corpus_texts and len(r["content"]) > 20]
    p50 = json.loads((work / "probes-p50.json").read_text(encoding="utf-8"))
    z33 = json.loads((work / "probes-z33.json").read_text(encoding="utf-8"))
    e50 = json.loads((work / "expected-p50.json").read_text(encoding="utf-8"))
    e33 = json.loads((work / "expected-z33.json").read_text(encoding="utf-8"))
    t33 = json.loads((work / "targets-z33.json").read_text(encoding="utf-8"))
    check_zero_overlap(z33, t33)
    probes = p50 + z33
    corpus_p, pool_p, probes_p = work / "corpus-slim.json", work / "pool.json", work / "probes-all.json"
    pool_p.write_text(json.dumps(pool), encoding="utf-8")
    probes_p.write_text(json.dumps(probes), encoding="utf-8")
    log(f"corpus {len(corpus)} · pool {len(pool)} · probes {len(probes)} · retention {RETENTION} · knobs: production defaults")

    emb = Embedder(work / "embeddings.json")

    def facets_for(path: Path, prefix: str, tag: str) -> dict:
        f = work / f"facets-{tag}.json"
        if not f.exists():
            out = subprocess.run([str(HARNESS), "facets", "--corpus", str(path), "--prefix", prefix], capture_output=True, text=True, encoding="utf-8", errors="replace", check=True)
            f.write_text(out.stdout, encoding="utf-8")
        d = json.loads(f.read_text(encoding="utf-8"))
        log(f"facets {tag}: {d['rows']} rows → {d['facets']} facets")
        return d["by_id"]
    fac_c = facets_for(corpus_p, "", "corpus")
    fac_p = facets_for(pool_p, "distractor: ", "pool")

    if a.prepare:
        texts = [r["content"] for r in corpus] + [f for fs in fac_c.values() for f in fs]
        texts += ["distractor: " + r["content"] for r in pool] + [f for fs in fac_p.values() for f in fs]
        texts += [p["query"] for p in probes]
        texts = list(dict.fromkeys(texts))
        log(f"preparing {len(texts)} texts ({sum(1 for t in texts if t not in emb.cache)} not yet embedded)")
        emb.batch(texts)
        log(f"embed cache: {len(emb.cache)} texts")
        return

    lo, hi = (int(x) for x in a.seeds.split(".."))
    tsv = work / "results.tsv"
    if not tsv.exists():
        tsv.write_text("seed\tarm\trecall10_p50\trecall10_z33\thits_p50\thits_z33\tphi_e001\tphi_hrm_before\tphi_hrm_after\tlive_after\ttotal_after\tinjected\tforgotten\twall_s\n", encoding="utf-8")

    for seed in range(lo, hi + 1):
        if "W" in a.arms:
            run = run_arm_w(seed, work, corpus_p, pool_p, probes_p, a.cycles, a.churn)
            s50, s33 = score(run["results"], e50), score(run["results"], e33)
            ph = phi_e001(emb, run["survivors"], work, f"w-{seed}")
            run.update({"score_p50": s50, "score_z33": s33, "phi_e001": ph})
            (work / "runs" / f"w-{seed}.json").write_text(json.dumps(run), encoding="utf-8")
            pb, pa = run["phi_hrm_before"], run["phi_hrm_after"]
            with tsv.open("a", encoding="utf-8") as f:
                f.write(f"{seed}\tW\t{s50['recall_at_10']:.4f}\t{s33['recall_at_10']:.4f}\t{s50['hits']}\t{s33['hits']}\t{ph['phi']:.4f}\t{pb['phi']}\t{pa['phi']}\t{run['live']}\t{run['total']}\t{run['distractors_injected']}\t{run['forgotten_total']}\t{run['wall_s']}\n")
            log(f"W{seed}: p50 {s50['recall_at_10']:.3f} ({s50['hits']}/50) · z33 {s33['recall_at_10']:.3f} ({s33['hits']}/33) · Φ_e001 {ph['phi']:.3f} · Φ_hrm {pb['phi']}→{pa['phi']} · live {run['live']} · forgot {run['forgotten_total']} · {run['wall_s']}s {run['embed_cache_miss_lines']}")
        if "V" in a.arms:
            t0 = time.time()
            run = run_arm_v(corpus=corpus, pool=pool, probes=probes, facets_corpus=fac_c, facets_pool=fac_p,
                            embed=emb, retention=RETENTION, cycles=a.cycles, churn=a.churn, seed=seed)
            s50, s33 = score(run["results"], e50), score(run["results"], e33)
            ph = phi_e001(emb, run["survivors"], work, f"v-{seed}")
            run.update({"score_p50": s50, "score_z33": s33, "phi_e001": ph, "wall_s": round(time.time() - t0, 1)})
            forgotten = sum(c["ghosted"] for c in run["cycle_rows"])
            (work / "runs" / f"v-{seed}.json").write_text(json.dumps(run), encoding="utf-8")
            with tsv.open("a", encoding="utf-8") as f:
                f.write(f"{seed}\tV\t{s50['recall_at_10']:.4f}\t{s33['recall_at_10']:.4f}\t{s50['hits']}\t{s33['hits']}\t{ph['phi']:.4f}\t-\t-\t{run['live']}\t{run['total']}\t{run['distractors_injected']}\t{forgotten}\t{run['wall_s']}\n")
            log(f"V{seed}: p50 {s50['recall_at_10']:.3f} ({s50['hits']}/50) · z33 {s33['recall_at_10']:.3f} ({s33['hits']}/33) · Φ_e001 {ph['phi']:.3f} · live {run['live']} · forgot {forgotten} · {run['wall_s']}s")
    emb.flush()
    log("done")


if __name__ == "__main__":
    main()
