#!/usr/bin/env python3
"""The two ground truths, built from the record and frozen with a hash.

E-007, "later recalled" (docs/experiments/E-007 §Fixed choices):
    python3 labels.py e007 --events events.jsonl --recalls recall-dump.json \
        --window-days 14 --min-queries 2 --out labels-e007.json
A held-out event is recalled-later if its memory_id appears in the memory_ids
of `KANNAKA.events.memory.<agent>.recall` events with at least --min-queries
distinct query_sha256 values within --window-days after its ts. Only events
from agents that published at least one .recall event in the dump are scored
(an unserved store is not a negative by construction); the rest are listed
as `unscored`.

E-004, "load-bearing" (docs/experiments/E-004 Amendment 1):
    python3 labels.py e004 --events events.jsonl --readings <dir> \
        --dump mem-dump.json --out labels-e004.json --probes probes.json
A held-out event is load-bearing if a reading's `cites` entry names it
(subject + seq) and the citation is VERIFIED: the message is in the dump and
its payload's canonical-JSON sha256 matches. This is bus_cite.py's rule
(NickFlach/assay), restated so the experiment carries its own check. The
probe for each label is the citation's `probe` text, else the reading's
top-level `question`; a label with neither gets no probe and is reported.

Both write the label list and its sha256 so the report can say what was
frozen. Standard library only.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path


def read_events(path: Path) -> list[dict]:
    return [json.loads(l) for l in path.read_text(encoding="utf-8").splitlines() if l.strip()]


def load_dump(path: Path) -> list[dict]:
    d = json.loads(path.read_text(encoding="utf-8"))
    return d.get("messages", []) if isinstance(d, dict) else d


def payload_sha256(payload) -> str:
    canon = json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(canon.encode("utf-8")).hexdigest()


def freeze(keys: list[str]) -> str:
    return hashlib.sha256("\n".join(sorted(keys)).encode("utf-8")).hexdigest()


def e007(a) -> int:
    events = read_events(a.events)
    recalls = [m for m in load_dump(a.recalls) if str(m.get("subject", "")).endswith(".recall")]
    serving = {str((m.get("payload") or {}).get("agent_id") or m["subject"].split(".")[3]) for m in recalls}
    by_mem: dict[str, list[tuple[int, str]]] = {}
    for m in recalls:
        p = m.get("payload") or {}
        for mid in p.get("memory_ids") or []:
            by_mem.setdefault(str(mid), []).append((int(m.get("ts", 0)), str(p.get("query_sha256", ""))))
    window_ms = a.window_days * 86_400_000
    labels, scored, unscored = [], [], []
    for e in events:
        if e["day"] < a.heldout_from:
            continue
        if e["agent"] not in serving:
            unscored.append(e["key"])
            continue
        scored.append(e["key"])
        hits = [(t, q) for t, q in by_mem.get(str(e.get("memory_id")), []) if e["ts"] <= t <= e["ts"] + window_ms]
        if len({q for _, q in hits}) >= a.min_queries:
            labels.append(e["key"])
    top = {}
    for m in recalls:
        q = str((m.get("payload") or {}).get("query_sha256", ""))
        top[q] = top.get(q, 0) + 1
    top_q = sorted(top.items(), key=lambda kv: -kv[1])[:10]
    out = {
        "experiment": "E-007", "rule": f">= {a.min_queries} distinct query_sha256 within {a.window_days} days",
        "serving_agents": sorted(serving), "scored": scored, "unscored_no_daemon": unscored,
        "labels": sorted(labels), "labels_sha256": freeze(labels),
        "recall_events": len(recalls),
        "top_query_hashes": [{"query_sha256": q, "events": n, "share": round(n / max(1, len(recalls)), 3)} for q, n in top_q],
    }
    a.out.write_text(json.dumps(out, indent=1), encoding="utf-8")
    print(f"[e007] {len(labels)} recalled-later of {len(scored)} scored ({len(unscored)} unscored, no daemon); "
          f"{len(recalls)} recall events; sha256 {out['labels_sha256'][:16]}", file=sys.stderr)
    return 0


def e004(a) -> int:
    events = read_events(a.events)
    by_key = {e["key"]: e for e in events if e["day"] >= a.heldout_from}
    index = {(m.get("subject"), int(m.get("seq", -1))): m for m in load_dump(a.dump)}
    labels, probes, verdicts = [], [], []
    for rp in sorted(Path(a.readings).glob("*.json")):
        try:
            reading = json.loads(rp.read_text(encoding="utf-8"))
        except Exception:
            continue
        for i, c in enumerate(reading.get("cites") or []):
            if not isinstance(c, dict) or "subject" not in c or "seq" not in c or "payload_sha256" not in c:
                verdicts.append({"reading": rp.name, "i": i, "verdict": "MALFORMED"})
                continue
            key = (c["subject"], int(c["seq"]))
            msg = index.get(key)
            if msg is None:
                verdicts.append({"reading": rp.name, "i": i, "verdict": "UNRESOLVED"})
                continue
            if payload_sha256(msg.get("payload")) != c["payload_sha256"]:
                verdicts.append({"reading": rp.name, "i": i, "verdict": "ALTERED"})
                continue
            ek = f"{key[0]}#{key[1]}"
            verdicts.append({"reading": rp.name, "i": i, "verdict": "VERIFIED", "key": ek, "in_heldout": ek in by_key})
            if ek in by_key and ek not in labels:
                labels.append(ek)
                probe = c.get("probe") or reading.get("question")
                if probe:
                    probes.append({"key": ek, "query": str(probe)})
    out = {
        "experiment": "E-004", "rule": "VERIFIED citation in a reading (bus_cite's check), event in days 21-30",
        "labels": sorted(labels), "labels_sha256": freeze(labels), "verdicts": verdicts,
        "labels_without_probe": sorted(set(labels) - {p["key"] for p in probes}),
    }
    a.out.write_text(json.dumps(out, indent=1), encoding="utf-8")
    if a.probes:
        a.probes.write_text(json.dumps(probes, indent=1, ensure_ascii=False), encoding="utf-8")
    n_ver = sum(1 for v in verdicts if v["verdict"] == "VERIFIED")
    print(f"[e004] {len(labels)} load-bearing held-out event(s) from {n_ver} verified citation(s) "
          f"({len(verdicts) - n_ver} not verified); {len(probes)} probe(s); sha256 {out['labels_sha256'][:16]}", file=sys.stderr)
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    p7 = sub.add_parser("e007")
    p7.add_argument("--events", type=Path, required=True)
    p7.add_argument("--recalls", type=Path, required=True, help="a dump holding the .recall events")
    p7.add_argument("--window-days", type=int, default=14)
    p7.add_argument("--min-queries", type=int, default=2)
    p7.add_argument("--heldout-from", type=int, default=21)
    p7.add_argument("--out", type=Path, required=True)
    p4 = sub.add_parser("e004")
    p4.add_argument("--events", type=Path, required=True)
    p4.add_argument("--readings", required=True, help="directory of assay readings (JSON)")
    p4.add_argument("--dump", type=Path, required=True, help="the bus dump the citations resolve against")
    p4.add_argument("--heldout-from", type=int, default=21)
    p4.add_argument("--out", type=Path, required=True)
    p4.add_argument("--probes", type=Path, default=None)
    a = ap.parse_args()
    return e007(a) if a.cmd == "e007" else e004(a)


if __name__ == "__main__":
    sys.exit(main())
