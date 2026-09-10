"""E-005 inputs from the E-001 work dir: one memory per line, and a probes TSV.

    python3 prepare.py --work ~/e001/work --out ~/e005

Writes <out>/memories.txt (615 lines; newlines inside a memory become spaces)
and <out>/probes.tsv (id, set, query, expected texts joined by " ||| ").
"""
import argparse
import json
from pathlib import Path


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--work", required=True)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    work, out = Path(a.work), Path(a.out)
    out.mkdir(parents=True, exist_ok=True)

    corpus = json.loads((work / "corpus-slim.json").read_text(encoding="utf-8"))
    by_id = {r["id"]: r["content"] for r in corpus}
    flat = lambda t: " ".join(t.split())
    (out / "memories.txt").write_text(
        "\n".join(flat(r["content"]) for r in corpus if r.get("content", "").strip()) + "\n",
        encoding="utf-8",
    )

    rows = []
    for name, pf, ef in (("p50", "probes-p50.json", "expected-p50.json"), ("z33", "probes-z33.json", "expected-z33.json")):
        probes = json.loads((work / pf).read_text(encoding="utf-8"))
        expected = json.loads((work / ef).read_text(encoding="utf-8"))
        for p in probes:
            pid = p["id"] if isinstance(p, dict) else p
            q = (p.get("query") or p.get("probe") or p.get("text")) if isinstance(p, dict) else ""
            exp = [flat(by_id[i]) for i in expected.get(pid, []) if i in by_id]
            if not q or not exp:
                continue
            rows.append(f"{pid}\t{name}\t{flat(q)}\t{' ||| '.join(exp)}")
    (out / "probes.tsv").write_text("\n".join(rows) + "\n", encoding="utf-8")
    print(f"{len(corpus)} memories; {len(rows)} probes -> {out}")


if __name__ == "__main__":
    main()
