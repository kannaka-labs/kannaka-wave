#!/usr/bin/env python3
"""Build the Wave's mind registry from this repository's own evidence.

    python3 architecture/registry.py --out architecture/wave-registry.json

One row per faculty that has an honest answer, in the crystal registry schema
kannaka-hdl's mind domain reads (`primitives: [{id, class, persistence,
noise_tolerance, material_id, evidence_level, behavioral_capabilities}]`).
Three rules, applied by the code and not by hand:

1. A faculty with a RUN experiment gets that experiment's measured number as
   `persistence`, parsed from the committed report; `noise_tolerance` is one
   minus the reported standard error; `evidence_level` is 2 when an
   independent replication is committed, else 1.
2. A faculty with NO experiment registered gets a row from its own tests,
   each executed here with `cargo test <name> -- --exact`; `persistence` is
   the fraction that passed. Evidence level 1: the build's own word.
3. A faculty whose experiment is registered but has NOT run gets no row.
   Strict mode then refuses to grow the Wave and names the faculty, which is
   what the architecture organ is for.

Capabilities are passed contracts: an experiment's verdict line, or a test that
passed when this script ran. Nothing here is typed in as a number.
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MATERIAL = "kannaka-wave"


def run_test(name: str) -> bool:
    r = subprocess.run(
        ["cargo", "test", "-q", name, "--", "--exact"],
        cwd=ROOT, capture_output=True, text=True,
    )
    return r.returncode == 0 and re.search(r"test result: ok\. 1 passed", r.stdout) is not None


def tests_row(cls: str, tests: list[str]) -> dict:
    caps = [{"name": t.rsplit("::", 1)[-1], "passed": run_test(t)} for t in tests]
    passed = sum(c["passed"] for c in caps)
    return {
        "id": f"wave.{cls.lower()}",
        "class": cls,
        "persistence": passed / len(caps),
        "noise_tolerance": 0.0,
        "material_id": MATERIAL,
        "evidence_level": 1,
        "behavioral_capabilities": caps,
        "source": "tests, executed by registry.py",
    }


def number(text: str, pattern: str) -> tuple[float, float]:
    """(value, standard error) from a report line like `V 0.576 ± 0.000`."""
    m = re.search(pattern, text)
    if not m:
        raise SystemExit(f"registry.py: pattern not found: {pattern}")
    return float(m.group(1)), float(m.group(2))


def recall_row() -> dict:
    rep = (ROOT / "experiments/e001/results/report.txt").read_text(encoding="utf-8")
    v, se = number(rep, r"recall@10 zero-overlap-33.*?V ([0-9.]+) ± ([0-9.]+)")
    promoted = "The substrate becomes arm V" in rep
    return {
        "id": "wave.recall",
        "class": "Recall",
        "persistence": v,
        "noise_tolerance": 1.0 - se,
        "material_id": MATERIAL,
        "evidence_level": 1,
        "behavioral_capabilities": [{"name": "zero_overlap_recall", "passed": promoted}],
        "source": "E-001 report, arm V, recall@10 zero-overlap",
    }


def forgetting_row() -> dict:
    tsv = (ROOT / "experiments/e001/results/results.tsv").read_text(encoding="utf-8").splitlines()
    head = tsv[0].split("\t")
    fr = []
    for line in tsv[1:]:
        d = dict(zip(head, line.split("\t")))
        if d.get("arm") == "V":
            fr.append(min(1.0, float(d["forgotten"]) / float(d["injected"])))
    if not fr:
        raise SystemExit("registry.py: no arm V rows in E-001 results")
    tests = [
        "store::tests::cap_evicts_least_recalled_then_least_important_and_promotion_is_immune",
        "store::tests::ttl_dissolves_expired_parents_and_their_facets",
    ]
    row = tests_row("Forgetting", tests)
    row["persistence"] = sum(fr) / len(fr)
    row["source"] = "E-001 results, arm V: rows forgotten per row injected (clipped at 1), plus the store's tests"
    return row


def speak_row() -> dict:
    rep = (ROOT / "experiments/e005/results/report.txt").read_text(encoding="utf-8")
    a, se = number(rep, r"anchored faithfulness\s+([0-9.]+) ± ([0-9.]+)")
    m = re.search(r"Floor = .*?([0-9]+\.[0-9]+)", rep)
    if not m:
        raise SystemExit("registry.py: E-005 report has no 'Floor = …' line")
    floor = float(m.group(1))
    rep_cpu = ROOT / "experiments/e005/results/replication-cpu/report.txt"
    replicated = rep_cpu.exists()
    caps = [{"name": "faithfulness_floor", "passed": a >= floor}]
    for t in [
        "voice::tests::speaks_from_a_reply_and_reports_a_failure_instead_of_inventing",
        "voice::tests::a_prompt_is_reduced_to_its_question_and_only_that",
    ]:
        caps.append({"name": t.rsplit("::", 1)[-1], "passed": run_test(t)})
    return {
        "id": "wave.speak",
        "class": "Speak",
        "persistence": a,
        "noise_tolerance": 1.0 - se,
        "material_id": MATERIAL,
        "evidence_level": 2 if replicated else 1,
        "behavioral_capabilities": caps,
        "source": "E-005 report, arm A anchored faithfulness; CPU replication committed" if replicated else "E-005 report",
    }


def experiment_unrun(doc: str) -> bool:
    """A registered experiment whose status line does not say it ran."""
    text = (ROOT / "docs/experiments" / doc).read_text(encoding="utf-8")
    status = re.search(r"\*\*Status:\*\*(.*?)(?:\n\*\*|\n\n)", text, re.S).group(1).lower()
    return "nothing has run" in status or "not run" in status or "pre-registered" in status and "result" not in status


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path, required=True)
    a = ap.parse_args()

    rows = [recall_row(), forgetting_row(), speak_row()]
    # Rule 2: no experiment registered for the rails.
    rows.append(tests_row("Decide", [
        "conscience::tests::decide_is_pure",
        "conscience::tests::an_ungranted_effector_is_refused_whatever_the_confidence",
        "conscience::tests::an_effector_not_cleared_for_all_five_refusals_goes_to_a_person",
        "conscience::tests::forbid_irreversible_refuses_and_outranks_escalation",
        "no_source_file_can_sign_spend_or_spawn",
        "only_the_one_http_client_opens_a_connection",
    ]))
    rows.append(tests_row("Audit", [
        "conscience::tests::the_chain_round_trips_and_every_tamper_is_found",
        "conscience::tests::a_tampered_chain_is_refused_before_anything_is_decided",
        "conscience::tests::an_entry_decided_against_a_stale_head_is_refused",
        "conscience::tests::every_entry_names_the_charter_it_was_decided_under",
    ]))
    # Rule 3: registered but unrun. No row; say so on stderr.
    waiting = {
        "Propose": "E-008-faithfulness-as-commutation-with-the-real-operators.md",
        "Predict": "E-004-does-surprise-pick-what-to-remember.md",
        "Surprise": "E-007-does-surprise-keep-what-gets-recalled.md",
    }
    for cls, doc in waiting.items():
        if experiment_unrun(doc):
            print(f"[registry] {cls}: no row; {doc.split('-')[0]}-{doc.split('-')[1]} is registered and has not run", file=sys.stderr)
        else:
            print(f"[registry] {cls}: {doc} reports a result but registry.py has no parser for it yet; no row", file=sys.stderr)

    out = {"schema": "crystal-registry", "built_by": "architecture/registry.py", "primitives": rows}
    a.out.write_text(json.dumps(out, indent=2) + "\n", encoding="utf-8")
    for r in rows:
        caps = ", ".join(c["name"] + ("" if c["passed"] else " (FAILED)") for c in r["behavioral_capabilities"])
        print(f"[registry] {r['class']:<10} persistence {r['persistence']:.3f}  evidence {r['evidence_level']}  {caps}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
