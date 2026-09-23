#!/usr/bin/env python3
"""E-004 / E-007 exporter: a bus dump → the harness's `events.jsonl`.

    python3 export.py --dump mem-1.json [--dump mem-2.json ...] \
        --day1 2026-09-23 --days 30 --exclude-agent grid-colony-one --out events.jsonl

Input: JSON dumps as the nats ninja-portal MCP `query_messages` returns them
(an array of {subject, seq, ts, payload}, or {"messages": [...]}), any number
of them, any overlap; duplicates by (subject, seq) are dropped. Only
`KANNAKA.events.memory.<agent>.remember` events are the world stream.

Output: one event per line in bus order (ts, then seq):
    {"key": "<subject>#<seq>", "agent": "...", "day": 1..N, "text": "...",
     "memory_id": "...", "ts": <unix ms>}
`day` is the 1-based UTC day index from --day1; events outside 1..--days are
dropped. --exclude-agent (repeatable) is Amendment 1's exclusion, applied by
agent_id before anything else.

The summary on stderr is the aggregate the record keeps (per-day counts, what
the exclusion removed); the dump itself is memory content and stays out of
the repository. Standard library only.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import sys
from pathlib import Path


def load_dumps(paths: list[Path]) -> list[dict]:
    seen = set()
    out = []
    for p in paths:
        d = json.loads(p.read_text(encoding="utf-8"))
        rows = d.get("messages", []) if isinstance(d, dict) else d
        for m in rows:
            k = (m.get("subject"), m.get("seq"))
            if k in seen or k[0] is None or k[1] is None:
                continue
            seen.add(k)
            out.append(m)
    out.sort(key=lambda m: (m.get("ts", 0), m.get("seq", 0)))
    return out


def day_index(ts_ms: int, day1: dt.date) -> int:
    d = dt.datetime.fromtimestamp(ts_ms / 1000, dt.timezone.utc).date()
    return (d - day1).days + 1


def export(rows: list[dict], day1: dt.date, days: int, exclude: set[str]) -> tuple[list[dict], dict]:
    events = []
    excluded = 0
    outside = 0
    not_remember = 0
    per_day: dict[int, int] = {}
    for m in rows:
        subject = str(m.get("subject", ""))
        if not subject.endswith(".remember") or ".events.memory." not in subject:
            not_remember += 1
            continue
        p = m.get("payload") or {}
        agent = str(p.get("agent_id") or subject.split(".")[3] if subject.count(".") >= 4 else "")
        if agent in exclude:
            excluded += 1
            continue
        day = day_index(int(m["ts"]), day1)
        if day < 1 or day > days:
            outside += 1
            continue
        text = str(p.get("content") or "").strip()
        if not text:
            continue
        per_day[day] = per_day.get(day, 0) + 1
        events.append({
            "key": f"{subject}#{m['seq']}",
            "agent": agent,
            "day": day,
            "text": text,
            "memory_id": p.get("memory_id"),
            "ts": int(m["ts"]),
        })
    summary = {
        "day1": day1.isoformat(),
        "days": days,
        "events": len(events),
        "excluded_by_agent": excluded,
        "outside_window": outside,
        "not_remember": not_remember,
        "per_day": [per_day.get(d, 0) for d in range(1, days + 1)],
        "zero_days": [d for d in range(1, days + 1) if per_day.get(d, 0) == 0],
        "train_days_1_20": sum(per_day.get(d, 0) for d in range(1, min(20, days) + 1)),
        "heldout_days_21_30": sum(per_day.get(d, 0) for d in range(21, days + 1)),
    }
    return events, summary


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dump", type=Path, action="append", required=True)
    ap.add_argument("--day1", required=True, help="UTC date of day 1, YYYY-MM-DD")
    ap.add_argument("--days", type=int, default=30)
    ap.add_argument("--exclude-agent", action="append", default=[])
    ap.add_argument("--out", type=Path, required=True)
    a = ap.parse_args()
    day1 = dt.date.fromisoformat(a.day1)
    rows = load_dumps(a.dump)
    events, summary = export(rows, day1, a.days, set(a.exclude_agent))
    with a.out.open("w", encoding="utf-8") as f:
        for e in events:
            f.write(json.dumps(e, ensure_ascii=False) + "\n")
    print(json.dumps(summary, indent=1), file=sys.stderr)
    if summary["zero_days"]:
        print(f"[export] WARNING: {len(summary['zero_days'])} zero day(s): {summary['zero_days']}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
