#!/usr/bin/env python3
"""E-004 / E-007 exporter: a bus dump → the harness's `events.jsonl`.

    python3 export.py --dump mem-1.json [--dump mem-2.json ...] \
        --day1 2026-09-23 --days 30 --exclude-agent grid-colony-one \
        [--store kannaka-prime=/path/to/store-copy] [--store-json AGENT=export.json] \
        --out events.jsonl

Input: JSON dumps as the nats ninja-portal MCP `query_messages` returns them
(an array of {subject, seq, ts, payload}, or {"messages": [...]}), any number
of them, any overlap; duplicates by (subject, seq) are dropped. Only
`KANNAKA.events.memory.<agent>.remember` events are the world stream.

Output: one event per line in bus order (ts, then seq):
    {"key": "<subject>#<seq>", "agent": "...", "day": 1..N, "text": "...",
     "memory_id": "...", "ts": <unix ms>, "text_source": "inline" | "store"}
`day` is the 1-based UTC day index from --day1; events outside 1..--days are
dropped. --exclude-agent (repeatable) is Amendment 1's exclusion, applied by
agent_id before anything else.

Ids-only events (kannaka-memory#1066). Since that release every write path
publishes a remember event, but only `kannaka remember` (via=cli) carries
`content` by default; every other origin sends `memory_id` and
`content_sha256` only, because the memory lane is anon-readable. For those,
the text is resolved by `memory_id` from a COPY of the agent's store, taken
on the export host at export time:

  --store AGENT=PATH   PATH is a store directory holding `kannaka.hrm` (or
                       the .hrm file itself). The exporter runs
                       `kannaka export-json --slim` once per store, against a
                       scratch data dir that holds only a symlink to that
                       .hrm, with KANNAKA_READONLY=1 and remember events off.
                       The scratch dir matters: a store's own config.toml
                       carries an absolute `hrm.path`, and the CLI prefers it
                       over KANNAKA_DATA_DIR, so pointing kannaka at the copy
                       itself would read the live store instead.
  --store-json AGENT=FILE  the saved stdout of that same command, for when
                       the export ran elsewhere.
  --kannaka BIN        the binary (default: `kannaka` on PATH).

A resolved text is used only if the SHA-256 of its UTF-8 bytes equals the
event's `content_sha256`, which is how kannaka computes it
(`remember_events::content_sha256`, over the stored content, untrimmed). A
store copy that has drifted from the event (the memory was rewritten, or the
copy is of the wrong store) therefore cannot put the wrong text in the
stream. The per-agent counts go in the summary; nothing is dropped silently.

The summary on stderr is the aggregate the record keeps (per-day counts, what
the exclusion removed); the dump itself is memory content and stays out of
the repository. Standard library only.
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

# Per-agent text accounting. Every remember event inside the window, after the
# exclusion, lands in exactly one of these.
COUNTERS = (
    "inline",               # the event carried `content`
    "resolved_by_id",       # ids-only, text from the store copy, hash verified
    "hash_mismatch",        # ids-only, memory found, text's sha256 != content_sha256: not used
    "unresolved_missing",   # ids-only, memory_id not in the store copy (pruned, forgotten, wrong store)
    "skipped_no_store",     # ids-only, no --store/--store-json for this agent
    "unverifiable_no_hash", # ids-only, no content_sha256 on the event: nothing to verify against
    "empty_text",           # content (inline or verified) is blank after strip
)


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


def content_sha256(text: str) -> str:
    """kannaka-memory's `remember_events::content_sha256`: hex SHA-256 of the
    content's UTF-8 bytes, exactly as stored (not trimmed, not normalised)."""
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def id_text_map(export_json) -> dict[str, str]:
    """`kannaka export-json [--slim]` output (a list of {id, content, ...}) →
    {id: content}."""
    rows = export_json.get("memories", []) if isinstance(export_json, dict) else export_json
    return {str(m["id"]): m.get("content") for m in rows if isinstance(m, dict) and "id" in m}


def _hrm_file(path: Path) -> Path:
    if path.is_file():
        return path
    hrm = path / "kannaka.hrm"
    if hrm.is_file():
        return hrm
    found = sorted(p.name for p in path.glob("*.hrm")) if path.is_dir() else []
    raise SystemExit(f"[export] {path}: no kannaka.hrm here" + (f" (found {found}; pass the file)" if found else ""))


def _refuse_live_store(path: Path) -> None:
    live = (Path.home() / ".kannaka").resolve()
    r = path.resolve()
    if r == live or live in r.parents:
        raise SystemExit(f"[export] {path} is the live store ({live}). Copy it first and pass the copy.")


def export_store(path: Path, kannaka_bin: str = "kannaka") -> dict[str, str]:
    """One `kannaka export-json --slim` over a store copy → {id: content}.

    Runs against a scratch KANNAKA_DATA_DIR holding only a symlink to the .hrm,
    so the copy's config.toml (whose absolute `hrm.path` would win over
    KANNAKA_DATA_DIR) is never read, with KANNAKA_READONLY=1 so nothing is
    persisted, and remember events off."""
    _refuse_live_store(path)
    hrm = _hrm_file(path)
    _refuse_live_store(hrm)
    with tempfile.TemporaryDirectory(prefix="e004-store-") as scratch:
        os.symlink(hrm.resolve(), Path(scratch) / "kannaka.hrm")
        enc = hrm.parent / ".encoder"
        if enc.is_file():
            os.symlink(enc.resolve(), Path(scratch) / ".encoder")
        env = dict(os.environ, KANNAKA_DATA_DIR=scratch, KANNAKA_READONLY="1", KANNAKA_EVENTS_REMEMBER="off")
        r = subprocess.run([kannaka_bin, "export-json", "--slim"], env=env, capture_output=True, text=True)
        if r.returncode != 0:
            raise SystemExit(f"[export] {kannaka_bin} export-json --slim on {hrm} failed ({r.returncode}): {r.stderr.strip()[-400:]}")
        try:
            return id_text_map(json.loads(r.stdout))
        except json.JSONDecodeError as e:
            raise SystemExit(f"[export] {kannaka_bin} export-json --slim on {hrm}: not JSON ({e})")


class StoreTexts:
    """id → text per agent, loaded once per agent and only if needed."""

    def __init__(self, stores: dict[str, Path] | None = None, store_json: dict[str, Path] | None = None,
                 kannaka_bin: str = "kannaka"):
        self.stores = dict(stores or {})
        self.store_json = dict(store_json or {})
        both = set(self.stores) & set(self.store_json)
        if both:
            raise SystemExit(f"[export] both --store and --store-json given for {sorted(both)}")
        self.kannaka_bin = kannaka_bin
        self._maps: dict[str, dict[str, str]] = {}

    def has(self, agent: str) -> bool:
        return agent in self.stores or agent in self.store_json

    def texts(self, agent: str) -> dict[str, str]:
        if agent not in self._maps:
            if agent in self.store_json:
                m = id_text_map(json.loads(self.store_json[agent].read_text(encoding="utf-8")))
            else:
                m = export_store(self.stores[agent], self.kannaka_bin)
            print(f"[export] {agent}: {len(m)} memories in the store copy", file=sys.stderr)
            self._maps[agent] = m
        return self._maps[agent]

    def unused(self) -> list[str]:
        return sorted((set(self.stores) | set(self.store_json)) - set(self._maps))


def resolve_text(p: dict, agent: str, store: StoreTexts | None) -> tuple[str | None, str]:
    """(text or None, counter). Inline content wins; otherwise the store copy,
    and only when the hash matches."""
    if "content" in p and p.get("content") is not None:
        text = str(p["content"]).strip()
        return (text, "inline") if text else (None, "empty_text")
    if store is None or not store.has(agent):
        return None, "skipped_no_store"
    want = p.get("content_sha256")
    if not want:
        return None, "unverifiable_no_hash"
    raw = store.texts(agent).get(str(p.get("memory_id")))
    if raw is None:
        return None, "unresolved_missing"
    if content_sha256(raw) != str(want).lower():
        return None, "hash_mismatch"
    text = raw.strip()
    return (text, "resolved_by_id") if text else (None, "empty_text")


def day_index(ts_ms: int, day1: dt.date) -> int:
    d = dt.datetime.fromtimestamp(ts_ms / 1000, dt.timezone.utc).date()
    return (d - day1).days + 1


def export(rows: list[dict], day1: dt.date, days: int, exclude: set[str],
           store: StoreTexts | None = None) -> tuple[list[dict], dict]:
    events = []
    text_by_agent: dict[str, dict[str, int]] = {}
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
        text, how = resolve_text(p, agent, store)
        counts = text_by_agent.setdefault(agent, dict.fromkeys(COUNTERS, 0))
        counts[how] += 1
        if text is None:
            continue
        per_day[day] = per_day.get(day, 0) + 1
        events.append({
            "key": f"{subject}#{m['seq']}",
            "agent": agent,
            "day": day,
            "text": text,
            "memory_id": p.get("memory_id"),
            "ts": int(m["ts"]),
            "text_source": "inline" if how == "inline" else "store",
        })
    summary = {
        "day1": day1.isoformat(),
        "days": days,
        "events": len(events),
        "excluded_by_agent": excluded,
        "outside_window": outside,
        "not_remember": not_remember,
        "text": {a: text_by_agent[a] for a in sorted(text_by_agent)},
        "text_total": {c: sum(v[c] for v in text_by_agent.values()) for c in COUNTERS},
        "per_day": [per_day.get(d, 0) for d in range(1, days + 1)],
        "zero_days": [d for d in range(1, days + 1) if per_day.get(d, 0) == 0],
        "train_days_1_20": sum(per_day.get(d, 0) for d in range(1, min(20, days) + 1)),
        "heldout_days_21_30": sum(per_day.get(d, 0) for d in range(21, days + 1)),
    }
    return events, summary


def parse_pairs(items: list[str], flag: str) -> dict[str, Path]:
    out = {}
    for it in items:
        agent, sep, path = it.partition("=")
        if not sep or not agent or not path:
            raise SystemExit(f"[export] {flag} expects AGENT=PATH, got {it!r}")
        if agent in out:
            raise SystemExit(f"[export] {flag} given twice for {agent}")
        out[agent] = Path(path).expanduser()
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dump", type=Path, action="append", required=True)
    ap.add_argument("--day1", required=True, help="UTC date of day 1, YYYY-MM-DD")
    ap.add_argument("--days", type=int, default=30)
    ap.add_argument("--exclude-agent", action="append", default=[])
    ap.add_argument("--store", action="append", default=[], metavar="AGENT=PATH",
                    help="a COPY of AGENT's store dir (or its .hrm), to resolve ids-only events by memory_id")
    ap.add_argument("--store-json", action="append", default=[], metavar="AGENT=FILE",
                    help="saved stdout of `kannaka export-json --slim` for AGENT's store")
    ap.add_argument("--kannaka", default=os.environ.get("KANNAKA_BIN", "kannaka"),
                    help="the kannaka binary used for --store (default: $KANNAKA_BIN or kannaka on PATH)")
    ap.add_argument("--out", type=Path, required=True)
    a = ap.parse_args()
    day1 = dt.date.fromisoformat(a.day1)
    store = StoreTexts(parse_pairs(a.store, "--store"), parse_pairs(a.store_json, "--store-json"), a.kannaka)
    rows = load_dumps(a.dump)
    events, summary = export(rows, day1, a.days, set(a.exclude_agent), store)
    if store.unused():
        summary["stores_unused"] = store.unused()
    with a.out.open("w", encoding="utf-8") as f:
        for e in events:
            f.write(json.dumps(e, ensure_ascii=False) + "\n")
    print(json.dumps(summary, indent=1), file=sys.stderr)
    for agent, c in summary["text"].items():
        print(f"[export] {agent}: " + ", ".join(f"{k}={c[k]}" for k in COUNTERS), file=sys.stderr)
    lost = {k: v for k, v in summary["text_total"].items() if k not in ("inline", "resolved_by_id") and v}
    if lost:
        print(f"[export] WARNING: remember events in the window without usable text: {lost}", file=sys.stderr)
    if summary["zero_days"]:
        print(f"[export] WARNING: {len(summary['zero_days'])} zero day(s): {summary['zero_days']}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
