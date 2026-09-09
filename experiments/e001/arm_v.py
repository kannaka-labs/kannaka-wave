#!/usr/bin/env python3
"""E-001 arm V: a plain vector store with the SAME encoder, the SAME facets and
the SAME forgetting policy as arm W, and nothing else.

Rows are parents and facets. A parent is a corpus memory or a distractor; its
facets are exactly the strings arm W stored (from `e001-harness facets`, which
calls kannaka_memory::facet::decompose). Recall is cosine over live rows with
facet hits resolved to their parent, keeping the best score per parent — the
ADR-0049 read-side rule. Triage mirrors ADR-0054 as implemented in
consolidation.rs::stage_retention_triage: per content-prefix rule, rows beyond
`cap` are ghosted lowest (retrieval_count, amplitude) first; rows with
retrieval_count >= promote_hits are immune. Ghosted rows leave recall and leave
the Φ population. Nothing here anneals, interferes, or spirals — that is the
point.

Invoked by run.py; not a CLI in its own right.
"""
from __future__ import annotations

import json
import math
import random
from dataclasses import dataclass, field


@dataclass
class Row:
    rid: str
    kind: str            # "parent" | "facet"
    parent: str | None   # parent rid for facets
    content: str
    vec: list[float]     # unit-normalised
    amplitude: float
    retrieval_count: int = 0
    ghost: bool = False


@dataclass
class VectorArm:
    """One arm-V store. `embed(text) -> unit vector` is injected so the
    orchestrator's cache is the single source of vectors for both arms."""
    embed: object
    retention: dict[str, dict]           # prefix -> {"cap": int|None, "ttl_days": float|None}
    promote_hits: int = 3
    rows: list[Row] = field(default_factory=list)
    _seq: int = 0

    def _new_id(self) -> str:
        self._seq += 1
        return f"v{self._seq:06d}"

    def absorb(self, content: str, amplitude: float, facets: list[str]) -> str:
        pid = self._new_id()
        self.rows.append(Row(pid, "parent", None, content, self.embed(content), amplitude))
        for f in facets:
            self.rows.append(Row(self._new_id(), "facet", pid, f, self.embed(f), amplitude))
        return pid

    def live(self) -> list[Row]:
        return [r for r in self.rows if not r.ghost]

    def recall(self, query: str, top_k: int = 10) -> list[tuple[str, float, str]]:
        """Returns [(parent_rid, score, parent_content)] best-per-parent, top_k."""
        q = self.embed(query)
        best: dict[str, float] = {}
        for r in self.live():
            s = _dot(q, r.vec)
            pid = r.parent or r.rid
            if s > best.get(pid, -2.0):
                best[pid] = s
        ranked = sorted(best.items(), key=lambda kv: kv[1], reverse=True)[:top_k]
        by_id = {r.rid: r for r in self.rows}
        out = []
        for pid, s in ranked:
            by_id[pid].retrieval_count += 1
            out.append((pid, s, by_id[pid].content))
        return out

    def dream(self) -> dict:
        """The forgetting policy, and only the forgetting policy."""
        ghosted = 0
        for prefix, rule in self.retention.items():
            cap = rule.get("cap")
            matching = [r for r in self.live() if r.content.startswith(prefix)]
            if cap is not None and len(matching) > cap:
                # Immune rows never count toward eviction; the rest go lowest-first.
                eligible = [r for r in matching if r.retrieval_count < self.promote_hits]
                eligible.sort(key=lambda r: (r.retrieval_count, r.amplitude))
                excess = len(matching) - cap
                for r in eligible[:excess]:
                    r.ghost = True
                    ghosted += 1
        return {"ghosted": ghosted, "live": len(self.live()), "total": len(self.rows)}

    def survivors(self) -> list[dict]:
        return [{"content": r.content, "amplitude": r.amplitude} for r in self.live()]


def _dot(a: list[float], b: list[float]) -> float:
    return sum(x * y for x, y in zip(a, b))


def run_arm_v(*, corpus: list[dict], pool: list[dict], probes: list[dict], facets_corpus: dict,
              facets_pool: dict, embed, retention: dict, cycles: int, churn: int, seed: int) -> dict:
    """Mirror of e001-harness wave-run, step for step."""
    arm = VectorArm(embed=embed, retention=retention)
    for row in corpus:
        amp = min(max(row.get("amplitude") or 0.6, 0.05), 1.0)
        arm.absorb(row["content"], amp, facets_corpus.get(row["id"], []))
    live0, total0 = len(arm.live()), len(arm.rows)

    order = list(range(len(pool)))
    random.Random(seed).shuffle(order)
    cursor = 0
    cycle_rows = []
    for c in range(1, cycles + 1):
        injected = 0
        for _ in range(churn):
            if cursor >= len(order):
                break
            d = pool[order[cursor]]
            cursor += 1
            text = "distractor: " + d["content"]
            arm.absorb(text, 0.5, facets_pool.get(d["id"], []))
            injected += 1
        before = len(arm.live())
        rep = arm.dream()
        cycle_rows.append({"cycle": c, "injected": injected, "live_before_dream": before,
                           "live_after_dream": rep["live"], "total_after_dream": rep["total"], "ghosted": rep["ghosted"]})

    by_content = {r["content"]: r["id"] for r in corpus}
    results, raw = {}, {}
    for p in probes:
        hits = arm.recall(p["query"], 10)
        results[p["id"]] = [by_content.get(c, f"nc:{pid}") for pid, _, c in hits]
        raw[p["id"]] = [s for _, s, _ in hits]

    return {
        "arm": "V", "seed": seed, "cycles": cycles, "churn": churn, "distractors_injected": cursor,
        "live_after_absorb": live0, "total_after_absorb": total0,
        "live": len(arm.live()), "total": len(arm.rows), "cycle_rows": cycle_rows,
        "results": results, "similarity": raw, "survivors": arm.survivors(),
    }
