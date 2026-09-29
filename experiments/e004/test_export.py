#!/usr/bin/env python3
"""Tests for export.py's text resolution (ids-only remember events, kannaka-memory#1066).

    python3 -m unittest discover -s experiments/e004 -p 'test_*.py' -v

Standard library only. The `kannaka` binary is replaced by a stub script that
checks how it was called and prints a fixture export. One optional test runs
the real binary against a throwaway store in a temp dir (never ~/.kannaka):
set E004_REAL_KANNAKA=/path/to/kannaka to enable it.
"""
from __future__ import annotations

import datetime as dt
import json
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import export as ex  # noqa: E402

DAY1 = dt.date(2026, 9, 23)
TS = int(dt.datetime(2026, 9, 24, 12, tzinfo=dt.timezone.utc).timestamp() * 1000)  # day 2

HERON = "the heron stands in the shallows"
UNICODE = "café naïve — unicode ✓ with trailing space "  # hashed untrimmed, as stored


def ev(seq: int, agent: str, memory_id: str, content=None, sha=None, drop_sha=False) -> dict:
    p = {"agent_id": agent, "memory_id": memory_id, "importance": 0.5, "modality": "Unknown",
         "via": "cli" if content is not None else "agent", "event_id": f"e{seq}", "schema_version": "1.0", "ts": TS}
    if content is not None:
        p["content"] = content
    if not drop_sha:
        p["content_sha256"] = sha if sha is not None else ex.content_sha256(content if content is not None else "")
    return {"subject": f"KANNAKA.events.memory.{agent}.remember", "seq": seq, "ts": TS, "payload": p}


STORE_EXPORT = [  # what `kannaka export-json --slim` prints: a list of rows
    {"id": "m-heron", "content": HERON, "amplitude": 0.5, "modality": "Unknown"},
    {"id": "m-uni", "content": UNICODE, "amplitude": 0.5, "modality": "Unknown"},
    {"id": "m-edited", "content": "the text as it is in the store now", "amplitude": 0.5},
]


def rows() -> list[dict]:
    return [
        ev(1, "cli-agent", "m-cli", content="written from the CLI"),                        # inline
        ev(2, "kannaka-prime", "m-heron", sha=ex.content_sha256(HERON)),                     # resolved
        ev(3, "kannaka-prime", "m-uni", sha=ex.content_sha256(UNICODE)),                     # resolved, untrimmed hash
        ev(4, "kannaka-prime", "m-edited", sha=ex.content_sha256("the text when it was published")),  # mismatch
        ev(5, "kannaka-prime", "m-pruned", sha=ex.content_sha256("forgotten")),              # missing
        ev(6, "no-store-agent", "m-x", sha=ex.content_sha256("anything")),                   # no store
        ev(7, "kannaka-prime", "m-heron", drop_sha=True),                                    # no hash
    ]


def zero_counts() -> dict:
    return dict.fromkeys(ex.COUNTERS, 0)


STUB = r"""#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
d = Path(os.environ["KANNAKA_DATA_DIR"])
log = os.environ.get("STUB_LOG")
if log:
    with open(log, "a") as f:
        f.write(json.dumps({"argv": sys.argv[1:], "readonly": os.environ.get("KANNAKA_READONLY"),
                            "events": os.environ.get("KANNAKA_EVENTS_REMEMBER"),
                            "files": sorted(p.name for p in d.iterdir()),
                            "hrm_is_link": (d / "kannaka.hrm").is_symlink()}) + "\n")
if sys.argv[1:] != ["export-json", "--slim"]:
    sys.exit(2)
sys.stdout.write((d / "kannaka.hrm").read_text())   # the fake .hrm holds the fixture export
"""


class Resolution(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.t = Path(self.tmp.name)
        self.stub = self.t / "kannaka-stub"
        self.stub.write_text(STUB)
        self.stub.chmod(self.stub.stat().st_mode | stat.S_IXUSR)
        self.store = self.t / "prime-copy"
        self.store.mkdir()
        (self.store / "kannaka.hrm").write_text(json.dumps(STORE_EXPORT))
        # A copied store carries config.toml with an absolute hrm.path; the resolver must not expose it.
        (self.store / "config.toml").write_text('[hrm]\npath = "/home/opc/.kannaka/kannaka.hrm"\n')
        self.json_export = self.t / "prime-export.json"
        self.json_export.write_text(json.dumps(STORE_EXPORT))
        self.log = self.t / "stub.log"

    def tearDown(self):
        self.tmp.cleanup()

    def check(self, events, summary):
        by_key = {e["key"].rsplit("#", 1)[1]: e for e in events}
        self.assertEqual(sorted(by_key), ["1", "2", "3"])
        self.assertEqual(by_key["1"]["text"], "written from the CLI")
        self.assertEqual(by_key["1"]["text_source"], "inline")
        self.assertEqual(by_key["2"]["text"], HERON)
        self.assertEqual(by_key["2"]["text_source"], "store")
        self.assertEqual(by_key["3"]["text"], UNICODE.strip())
        prime = summary["text"]["kannaka-prime"]
        self.assertEqual(prime, dict(zero_counts(), resolved_by_id=2, hash_mismatch=1,
                                     unresolved_missing=1, unverifiable_no_hash=1))
        self.assertEqual(summary["text"]["cli-agent"], dict(zero_counts(), inline=1))
        self.assertEqual(summary["text"]["no-store-agent"], dict(zero_counts(), skipped_no_store=1))
        self.assertEqual(summary["events"], 3)
        self.assertEqual(sum(summary["text_total"].values()), 7, "every in-window event is counted once")

    def test_sha256_matches_kannaka(self):
        # remember_events::tests::content_sha256_is_hex_sha256
        self.assertEqual(ex.content_sha256("abc"),
                         "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        self.assertNotEqual(ex.content_sha256(UNICODE), ex.content_sha256(UNICODE.strip()))

    def test_no_store_nothing_resolved_nothing_silent(self):
        events, summary = ex.export(rows(), DAY1, 30, set())
        self.assertEqual([e["memory_id"] for e in events], ["m-cli"])
        self.assertEqual(summary["text"]["kannaka-prime"], dict(zero_counts(), skipped_no_store=5))
        self.assertEqual(summary["text_total"]["skipped_no_store"], 6)

    def test_store_json(self):
        store = ex.StoreTexts(store_json={"kannaka-prime": self.json_export})
        self.check(*ex.export(rows(), DAY1, 30, set(), store))

    def test_store_dir_via_stub_binary(self):
        store = ex.StoreTexts(stores={"kannaka-prime": self.store}, kannaka_bin=str(self.stub))
        with mock.patch.dict(os.environ, {"STUB_LOG": str(self.log)}):
            self.check(*ex.export(rows(), DAY1, 30, set(), store))
        calls = [json.loads(l) for l in self.log.read_text().splitlines()]
        self.assertEqual(len(calls), 1, "one bulk export per store, not one call per id")
        c = calls[0]
        self.assertEqual(c["argv"], ["export-json", "--slim"])
        self.assertEqual(c["readonly"], "1")
        self.assertEqual(c["events"], "off")
        self.assertTrue(c["hrm_is_link"])
        self.assertNotIn("config.toml", c["files"])

    def test_store_file_path_accepted(self):
        store = ex.StoreTexts(stores={"kannaka-prime": self.store / "kannaka.hrm"}, kannaka_bin=str(self.stub))
        self.check(*ex.export(rows(), DAY1, 30, set(), store))

    def test_store_loaded_only_when_needed(self):
        store = ex.StoreTexts(stores={"kannaka-prime": self.store}, kannaka_bin=str(self.stub))
        with mock.patch.dict(os.environ, {"STUB_LOG": str(self.log)}):
            ex.export(rows()[:1], DAY1, 30, set(), store)
        self.assertFalse(self.log.exists())
        self.assertEqual(store.unused(), ["kannaka-prime"])

    def test_excluded_and_outside_window_not_counted(self):
        store = ex.StoreTexts(store_json={"kannaka-prime": self.json_export})
        _, summary = ex.export(rows(), DAY1, 30, {"kannaka-prime"}, store)
        self.assertNotIn("kannaka-prime", summary["text"])
        self.assertEqual(summary["excluded_by_agent"], 5)
        _, summary = ex.export(rows(), DAY1 + dt.timedelta(days=5), 30, set(), store)
        self.assertEqual(summary["text"], {})
        self.assertEqual(summary["outside_window"], 7)

    def test_refuses_live_store(self):
        home = self.t / "home"
        (home / ".kannaka").mkdir(parents=True)
        (home / ".kannaka" / "kannaka.hrm").write_text("[]")
        with mock.patch.object(Path, "home", return_value=home):
            with self.assertRaises(SystemExit):
                ex.export_store(home / ".kannaka", str(self.stub))
            with self.assertRaises(SystemExit):
                ex.export_store(home / ".kannaka" / "kannaka.hrm", str(self.stub))

    def test_not_a_store(self):
        with self.assertRaises(SystemExit):
            ex.export_store(self.t, str(self.stub))

    def test_cli_summary_on_stderr(self):
        dump = self.t / "dump.json"
        dump.write_text(json.dumps({"messages": rows()}))
        out = self.t / "events.jsonl"
        r = subprocess.run([sys.executable, str(HERE / "export.py"), "--dump", str(dump), "--day1", DAY1.isoformat(),
                            "--store", f"kannaka-prime={self.store}", "--kannaka", str(self.stub), "--out", str(out)],
                           capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(len(out.read_text().splitlines()), 3)
        self.assertIn("[export] kannaka-prime: inline=0, resolved_by_id=2, hash_mismatch=1, unresolved_missing=1, "
                      "skipped_no_store=0", r.stderr)
        self.assertIn("[export] no-store-agent: inline=0, resolved_by_id=0, hash_mismatch=0, unresolved_missing=0, "
                      "skipped_no_store=1", r.stderr)
        self.assertIn("WARNING: remember events in the window without usable text", r.stderr)

    def test_bad_pairs(self):
        with self.assertRaises(SystemExit):
            ex.parse_pairs(["kannaka-prime"], "--store")
        with self.assertRaises(SystemExit):
            ex.StoreTexts({"a": Path("x")}, {"a": Path("y")})


def recall_ev(seq: int, agent: str, top_k, query: str = "what is the weather", ids=("m-heron", "m-uni"),
              drop_top_k=False) -> dict:
    """A `.recall` event as `swarm serve` publishes it (kannaka-memory nats.rs, EventPayload::MemoryRecall):
    ids and scores, top_k, the query's sha256, via=daemon, and the envelope. Never the query or content."""
    p = {"agent_id": agent, "memory_ids": list(ids), "similarities": [0.81, 0.64][:len(ids)],
         "query_sha256": ex.content_sha256(query), "via": "daemon",
         "event_id": f"r{seq}", "schema_version": "1.0", "ts": TS}
    if not drop_top_k:
        p["top_k"] = top_k
    return {"subject": f"KANNAKA.events.memory.{agent}.recall", "seq": seq, "ts": TS, "payload": p}


OBSERVATORY_Q = "what has kannaka been thinking about"
RESPONDER_Q = "Nick: hey kannaka, what's playing?"
PROBE_Q = "rollout probe"


def hints() -> dict:
    return {ex.content_sha256(OBSERVATORY_Q): "observatory", ex.content_sha256(RESPONDER_Q): "responder",
            ex.content_sha256(PROBE_Q): "operator-probe"}


class CallerClass(unittest.TestCase):
    """caller_class: hint table > unknown, no payload heuristic. One test per class, the override, the collision."""

    def test_top_k_alone_never_classifies(self):
        for top_k in (1, 2, 10):
            self.assertEqual(ex.caller_class(recall_ev(1, "kannaka-prime", top_k)["payload"]), "unknown",
                             "operator-probe comes only from the hint table")

    def test_observatory_only_via_hint(self):
        e = recall_ev(1, "kannaka-prime", 5, query=OBSERVATORY_Q)["payload"]
        self.assertEqual(ex.caller_class(e, hints()), "observatory")
        self.assertEqual(ex.caller_class(e), "unknown", "the CLI default top_k=5 alone proves nothing")

    def test_responder_only_via_hint(self):
        e = recall_ev(1, "kannaka-prime", 5, query=RESPONDER_Q)["payload"]
        self.assertEqual(ex.caller_class(e, hints()), "responder")
        self.assertEqual(ex.caller_class(e), "unknown")
        # The responder's target agent is configurable; a hint still names it.
        self.assertEqual(ex.caller_class(recall_ev(2, "other-agent", 5, query=RESPONDER_Q)["payload"], hints()),
                         "responder")

    def test_unknown(self):
        for top_k in (1, 2, 8, 10, 100, 0, -1, None, "many", 5.5, 1.0):
            self.assertEqual(ex.caller_class(recall_ev(1, "kannaka-prime", top_k)["payload"]), "unknown", top_k)
        self.assertEqual(ex.caller_class(recall_ev(2, "kannaka-prime", 8, drop_top_k=True)["payload"]), "unknown")
        self.assertEqual(ex.caller_class({}), "unknown")
        self.assertEqual(ex.caller_class(recall_ev(3, "kannaka-prime", True)["payload"]), "unknown",
                         "bool is not a top_k")

    def test_hint_overrides_heuristic(self):
        e = recall_ev(1, "kannaka-prime", 1, query=RESPONDER_Q)["payload"]
        self.assertEqual(ex.caller_class(e), "unknown")
        self.assertEqual(ex.caller_class(e, hints()), "responder")
        h = {ex.content_sha256(PROBE_Q).upper(): "operator-probe"}
        self.assertEqual(ex.caller_class(recall_ev(2, "kannaka-prime", 8, query=PROBE_Q)["payload"],
                                         ex.load_caller_hints(self._write(h))), "operator-probe",
                         "hint keys are matched case-insensitively once loaded")
        self.assertEqual(ex.caller_class(recall_ev(3, "kannaka-prime", 1, query="unhinted")["payload"], hints()),
                         "unknown", "a hint table that does not mention the hash changes nothing")

    def test_top_k_5_collision_is_unknown(self):
        # responder (5), observatory via `kannaka recall --remote` (CLI default 5) and
        # `swarm brief --peers` (5) are indistinguishable, for prime and for any other agent.
        self.assertEqual(ex.caller_class(recall_ev(1, "kannaka-prime", 5)["payload"]), "unknown")
        self.assertEqual(ex.caller_class(recall_ev(2, "some-peer", 5)["payload"]), "unknown")
        self.assertEqual(ex.caller_class(recall_ev(3, "kannaka-prime", 5)["payload"], hints()), "unknown",
                         "an unrelated hint table does not break the tie")

    def _write(self, obj) -> Path:
        t = tempfile.NamedTemporaryFile("w", suffix=".json", delete=False)
        self.addCleanup(os.unlink, t.name)
        json.dump(obj, t)
        t.close()
        return Path(t.name)

    def test_load_caller_hints_refuses_bad_input(self):
        good = ex.load_caller_hints(self._write({ex.content_sha256(PROBE_Q).upper(): "operator-probe"}))
        self.assertEqual(good, {ex.content_sha256(PROBE_Q): "operator-probe"})
        with self.assertRaises(SystemExit):
            ex.load_caller_hints(self._write({"rollout probe": "operator-probe"}))  # the text, not its hash
        with self.assertRaises(SystemExit):
            ex.load_caller_hints(self._write({ex.content_sha256(PROBE_Q): "mcp"}))  # not a class
        with self.assertRaises(SystemExit):
            ex.load_caller_hints(self._write([ex.content_sha256(PROBE_Q)]))


class RecallExport(unittest.TestCase):
    def rows(self) -> list[dict]:
        return rows() + [
            recall_ev(10, "kannaka-prime", 5, query=OBSERVATORY_Q),
            recall_ev(11, "kannaka-prime", 5, query=RESPONDER_Q),
            recall_ev(12, "kannaka-prime", 1, query=PROBE_Q),
            recall_ev(13, "kannaka-prime", 5, query="a DM nobody tagged"),
            recall_ev(14, "kannaka-prime", 10, query="mcp"),
            recall_ev(15, "grid-colony-one", 2, query=PROBE_Q),
            recall_ev(16, "peer-b", 5, query="brief topic"),
        ]

    def test_rows_columns_and_summary(self):
        recalls, s = ex.export_recalls(self.rows(), DAY1, {"grid-colony-one"}, hints())
        self.assertEqual([r["key"].rsplit("#", 1)[1] for r in recalls], ["10", "11", "12", "13", "14", "16"])
        self.assertTrue(all(tuple(r) == ex.RECALL_COLUMNS for r in recalls), "column order is fixed, caller_class last")
        self.assertEqual([r["caller_class"] for r in recalls],
                         ["observatory", "responder", "operator-probe", "unknown", "unknown", "unknown"])
        r = recalls[0]
        self.assertEqual((r["agent"], r["ts"], r["day"], r["top_k"], r["via"]), ("kannaka-prime", TS, 2, 5, "daemon"))
        self.assertEqual(r["memory_ids"], ["m-heron", "m-uni"])
        self.assertEqual(r["query_sha256"], ex.content_sha256(OBSERVATORY_Q))
        for k in ("query", "content", "results"):
            self.assertNotIn(k, r)
        self.assertEqual(s["events"], 6)
        self.assertEqual(s["excluded_by_agent"], 1)
        self.assertEqual(s["hinted"], 3)
        self.assertEqual(s["callers"]["kannaka-prime"],
                         {"observatory": 1, "responder": 1, "operator-probe": 1, "unknown": 2})
        self.assertEqual(s["callers"]["peer-b"], {"observatory": 0, "responder": 0, "operator-probe": 0, "unknown": 1})
        self.assertEqual(s["callers_total"], {"observatory": 1, "responder": 1, "operator-probe": 1, "unknown": 3})
        self.assertEqual(sum(s["callers_total"].values()), 6, "every kept recall lands in exactly one class")

    def test_no_window_and_no_hints(self):
        recalls, s = ex.export_recalls(self.rows(), DAY1 + dt.timedelta(days=40), set(), None)
        self.assertEqual(len(recalls), 7, "recall rows are never dropped by the day window")
        self.assertTrue(all(r["day"] < 1 for r in recalls))
        self.assertEqual(s["hinted"], 0)
        self.assertEqual(s["callers_total"], {"observatory": 0, "responder": 0, "operator-probe": 0, "unknown": 7})

    def test_remember_export_unchanged_by_recall_events(self):
        events, summary = ex.export(self.rows(), DAY1, 30, set())
        self.assertEqual([e["memory_id"] for e in events], ["m-cli"])
        self.assertEqual(summary["not_remember"], 7)
        self.assertNotIn("caller_class", events[0])

    def test_cli_recalls_out_and_hints(self):
        with tempfile.TemporaryDirectory() as t:
            t = Path(t)
            (t / "dump.json").write_text(json.dumps({"messages": self.rows()}))
            (t / "hints.json").write_text(json.dumps(hints()))
            r = subprocess.run([sys.executable, str(HERE / "export.py"), "--dump", str(t / "dump.json"),
                                "--day1", DAY1.isoformat(), "--exclude-agent", "grid-colony-one",
                                "--out", str(t / "events.jsonl"), "--recalls-out", str(t / "recalls.jsonl"),
                                "--caller-hints", str(t / "hints.json")], capture_output=True, text=True)
            self.assertEqual(r.returncode, 0, r.stderr)
            lines = [json.loads(l) for l in (t / "recalls.jsonl").read_text().splitlines()]
            self.assertEqual(len(lines), 6)
            self.assertEqual(list(lines[0]), list(ex.RECALL_COLUMNS))
            self.assertIn("[export] kannaka-prime recalls: observatory=1, responder=1, operator-probe=1, unknown=2",
                          r.stderr)
            self.assertIn('"recalls"', r.stderr)
            self.assertEqual(len((t / "events.jsonl").read_text().splitlines()), 1)
            # No --recalls-out: the file is not written, the summary still counts.
            r = subprocess.run([sys.executable, str(HERE / "export.py"), "--dump", str(t / "dump.json"),
                                "--day1", DAY1.isoformat(), "--out", str(t / "events2.jsonl")],
                               capture_output=True, text=True)
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertIn("[export] grid-colony-one recalls: observatory=0, responder=0, operator-probe=0, unknown=1",
                          r.stderr)
            self.assertFalse((t / "recalls2.jsonl").exists())


@unittest.skipUnless(os.environ.get("E004_REAL_KANNAKA"), "set E004_REAL_KANNAKA to run against a real binary")
class RealKannaka(unittest.TestCase):
    """Builds a throwaway store in a temp dir with the real CLI, then resolves from a copy of it."""

    def test_round_trip(self):
        kb = os.environ["E004_REAL_KANNAKA"]
        with tempfile.TemporaryDirectory() as t:
            t = Path(t)
            live = t / "store"
            live.mkdir()
            env = dict(os.environ, KANNAKA_DATA_DIR=str(live), HOME=str(t), KANNAKA_EVENTS_REMEMBER="off",
                       NATS_URL="nats://127.0.0.1:1", KANNAKA_NATS_URL="nats://127.0.0.1:1")
            ids = {}
            for text in (HERON, UNICODE):
                r = subprocess.run([kb, "remember", text], env=env, capture_output=True, text=True, timeout=120)
                self.assertEqual(r.returncode, 0, r.stderr)
                ids[r.stdout.strip().splitlines()[-1]] = text
            copy = t / "copy"
            subprocess.run(["cp", "-R", str(live), str(copy)], check=True)
            before = {p.name: p.read_bytes() for p in copy.iterdir() if p.is_file()}
            with mock.patch.object(Path, "home", return_value=t):
                got = ex.export_store(copy, kb)
            for mid, text in ids.items():
                self.assertEqual(got[mid], text)
                self.assertEqual(ex.content_sha256(got[mid]), ex.content_sha256(text))
            self.assertEqual(before, {p.name: p.read_bytes() for p in copy.iterdir() if p.is_file()},
                             "the store copy is not written")



class RealHintTable(unittest.TestCase):
    """hints/kannaka-prime-2026-09-29.json: every .recall event kannaka-prime had served by 2026-09-28
    (8 events, seq 4948-5112), all rollout probes or Kannaka's one MCP call. Hashes only, no query text."""

    def test_loads_and_classifies(self):
        path = Path(__file__).with_name("hints") / "kannaka-prime-2026-09-29.json"
        hints = ex.load_caller_hints(path)
        self.assertEqual(len(hints), 8)
        self.assertTrue(all(v == "operator-probe" for v in hints.values()))
        ev = recall_ev(5112, "kannaka-prime", 2)
        ev["payload"]["query_sha256"] = "d2c3b2e2a316385d54fd47be888efa17abc223ceea00319531ee0d2a74fd7a0e"  # seq 5112
        self.assertEqual(ex.caller_class(ev["payload"], hints), "operator-probe")
        self.assertEqual(ex.caller_class(recall_ev(9999, "kannaka-prime", 5)["payload"], hints), "unknown",
                         "a ninth prime event not in the table is the first organic recall, and stays unknown")

if __name__ == "__main__":
    unittest.main()
