#!/usr/bin/env python3
"""E-008 runner: every probe through three passes, each with its own audit chain.

    A1  plan then charter   (wave ask --propose)
    A2  plan then charter again, for the noise floor
    B   charter then plan   (wave ask --propose --charter-first)

    python run.py --wave ./wave --store data/store.kwave --probes data/probes.tsv \
                  --charter charter.kwc --out out/ [--only p01,p02]

Writes out/results.tsv (one row per probe: the effector each pass proposed or NONE, the
verdict, the raw reply), out/answers-<pass>.txt, and out/audit-<pass>.kwa (the rails'
record for each pass). Standard library only. The voice is whatever KWAVE_VOICE_MODEL and
KWAVE_OLLAMA_HOST/PORT point at; the report records them.
"""
from __future__ import annotations

import argparse
import hashlib
import os
import re
import subprocess
import sys
from pathlib import Path

PASSES = (("A1", []), ("A2", []), ("B", ["--charter-first"]))


def sha16(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()[:16]


def probes(path: Path) -> list[tuple[str, str]]:
    out = []
    for line in path.read_text(encoding="utf-8").splitlines():
        parts = line.split("\t")
        if len(parts) >= 3 and parts[0] != "id":
            out.append((parts[0], parts[2]))
    return out


def one(wave: str, env: dict, prompt: str, extra: list[str]) -> dict:
    cmd = [wave, "ask", prompt, "--propose", "--show-recall", *extra]
    r = subprocess.run(cmd, capture_output=True, text=True, env=env)
    out, err = r.stdout, r.stderr
    raw = ""
    m = re.search(r"voice's reply to the proposal prompt:\n(.*?)(?:\n\n|\Z)", err, re.S)
    if m:
        raw = m.group(1).strip()
    effector, verdict = "NONE", "-"
    m = re.search(r"^#\d+ (PACKAGE|REFUSED|ESCALATE)\s+(\S+)", out, re.M)
    if m:
        verdict, effector = m.group(1), m.group(2)
    answer = out.split("\nproposal", 1)[0].strip()
    return {"effector": effector, "verdict": verdict, "raw": raw, "answer": answer, "rc": r.returncode}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--wave", required=True)
    ap.add_argument("--store", required=True, type=Path)
    ap.add_argument("--probes", required=True, type=Path)
    ap.add_argument("--charter", required=True, type=Path)
    ap.add_argument("--out", required=True, type=Path)
    ap.add_argument("--only", default="")
    a = ap.parse_args()
    a.out.mkdir(parents=True, exist_ok=True)
    ps = probes(a.probes)
    if a.only:
        keep = set(a.only.split(","))
        ps = [p for p in ps if p[0] in keep]
    manifest = [
        f"store: {sha16(a.store)}  probes: {sha16(a.probes)}  charter: {sha16(a.charter)}",
        f"voice: {os.environ.get('KWAVE_VOICE_MODEL', 'kannaka-brain-7b-v1')} at "
        f"{os.environ.get('KWAVE_OLLAMA_HOST', '127.0.0.1')}:{os.environ.get('KWAVE_OLLAMA_PORT', '11434')}",
        f"probes run: {len(ps)}",
    ]
    (a.out / "run-manifest.txt").write_text("\n".join(manifest) + "\n", encoding="utf-8")
    print("\n".join(manifest))
    rows = ["id\t" + "\t".join(f"{p}_effector\t{p}_verdict" for p, _ in PASSES)]
    answers = {p: [] for p, _ in PASSES}
    for pid, q in ps:
        cells = [pid]
        for name, extra in PASSES:
            # A fresh store copy per pass keeps recall counts from coupling passes;
            # a fresh audit chain per pass keeps the rails' history separate.
            store = a.out / f"store-{name}.kwave"
            if not store.exists():
                store.write_bytes(a.store.read_bytes())
            env = dict(
                os.environ,
                KWAVE_STORE=str(store),
                KWAVE_CHARTER=str(a.charter),
                KWAVE_AUDIT=str(a.out / f"audit-{name}.kwa"),
            )
            r = one(a.wave, env, q, extra)
            cells += [r["effector"], r["verdict"]]
            answers[name].append(f"### {pid}\n{r['answer']}\n--- reply:\n{r['raw']}\n")
            print(f"{pid} {name}: {r['effector']} {r['verdict']}" + ("" if r["rc"] == 0 else f"  (rc {r['rc']})"))
        rows.append("\t".join(cells))
        (a.out / "results.tsv").write_text("\n".join(rows) + "\n", encoding="utf-8")
        for name, _ in PASSES:
            (a.out / f"answers-{name}.txt").write_text("\n".join(answers[name]), encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
