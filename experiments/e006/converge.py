"""E-006 first reading: do citizens on one digest share more phrasing than citizens who do not?

Measure, per ordered author pair (A,B): the fraction of A's distinct word 4-grams that also
occur in B ("shared 4-gram mass", directional, normalised by A's own size so a small author is
not penalised). Groups: BRAIN (citizens on kannaka-brain digest 67ed8d0a3526 per Lab Note No 4),
CONTROL (unrelated prolific agents), FLEET (a cluster of one-word-name agents that publish in lockstep;
model unknown). Bootstrap over documents gives a CI per group mean.
"""
import json, re, random, itertools, statistics, sys
sys.stdout.reconfigure(encoding="utf-8")
corpus = json.load(open("corpus.json", encoding="utf-8"))
BRAIN = ["Kannaka", "gossipghost", "The Archivist", "Ghost Signal", "Rogue Agent"]
MIXED = ["0xSCADA-QE"]           # identity shared by a Claude-driven writer and a brain twin; reported, not grouped
CONTROL = ["Noah", "VeeBot2", "Tiramisu", "Xuan"]
FLEET = ["York","Moss","Maya","Blake","Cedar","Haze","Rowan","Delta","Kai","Heath","Ren","Dex","Fern","Sable"]
N = 4
def toks(t):
    t = t.lower(); t = re.sub(r"[^a-z0-9' ]+", " ", t); return [w for w in t.split() if w]
def grams(docs):
    s = set()
    for d in docs:
        w = toks(d["content"])
        s.update(tuple(w[i:i+N]) for i in range(len(w)-N+1))
    return s
def docs(name): return corpus.get(name, {}).get("docs", [])
def share(a, b):
    ga, gb = grams(a), grams(b)
    if not ga or not gb: return None
    return len(ga & gb) / len(ga)
def pair_stats(names, label, boot=200, seed=1):
    rnd = random.Random(seed); vals = {}
    for a, b in itertools.permutations(names, 2):
        v = share(docs(a), docs(b))
        if v is not None: vals[(a, b)] = v
    if not vals: return
    mean = statistics.fmean(vals.values())
    # bootstrap: resample documents within each author
    bs = []
    for _ in range(boot):
        rs = []
        for a, b in vals:
            da, db = docs(a), docs(b)
            ra = [rnd.choice(da) for _ in da]; rb = [rnd.choice(db) for _ in db]
            v = share(ra, rb)
            if v is not None: rs.append(v)
        bs.append(statistics.fmean(rs))
    bs.sort(); lo, hi = bs[int(0.025*boot)], bs[int(0.975*boot)-1]
    print(f"{label:28s} pairs {len(vals):3d}  mean shared-4gram mass {mean:.4f}  95% boot [{lo:.4f}, {hi:.4f}]")
    return vals, mean
print("corpus sizes (docs / 4-grams):")
for n in BRAIN + MIXED + CONTROL + FLEET:
    d = docs(n); print(f"  {n:16s} {len(d):3d} docs  {len(grams(d)):6d} 4-grams")
print()
within_brain = pair_stats(BRAIN, "BRAIN x BRAIN (same digest)")
within_ctrl = pair_stats(CONTROL, "CONTROL x CONTROL")
within_fleet = pair_stats(FLEET, "FLEET x FLEET")
# cross groups
def cross(A, B, label):
    vals = {}
    for a in A:
        for b in B:
            v = share(docs(a), docs(b))
            if v is not None: vals[(a,b)] = v
    if vals: print(f"{label:28s} pairs {len(vals):3d}  mean shared-4gram mass {statistics.fmean(vals.values()):.4f}")
    return vals
cross(BRAIN, CONTROL, "BRAIN -> CONTROL")
cross(CONTROL, BRAIN, "CONTROL -> BRAIN")
cross(BRAIN, FLEET, "BRAIN -> FLEET")
cross(MIXED, BRAIN, "QE(mixed) -> BRAIN")
cross(MIXED, CONTROL, "QE(mixed) -> CONTROL")
print()
if within_brain:
    print("BRAIN pairs, highest first:")
    for (a,b),v in sorted(within_brain[0].items(), key=lambda kv:-kv[1])[:8]: print(f"  {a:14s} -> {b:14s} {v:.4f}")
# refrains: 5-grams shared by >=2 brain authors that appear in no control author
def gramsN(ds, n):
    s=set()
    for d in ds:
        w=toks(d["content"]); s.update(tuple(w[i:i+n]) for i in range(len(w)-n+1))
    return s
G5 = {n: gramsN(docs(n),5) for n in BRAIN+CONTROL+FLEET+MIXED}
from collections import Counter
c = Counter()
for n in BRAIN:
    for g in G5[n]: c[g]+=1
ctrl_all = set().union(*(G5[n] for n in CONTROL+FLEET))
refrains = [(g,k) for g,k in c.items() if k>=2 and g not in ctrl_all]
print(f"\n5-grams shared by >=2 brain citizens and absent from every control: {len(refrains)}")
for g,k in sorted(refrains, key=lambda x:-x[1])[:25]: print(f"  x{k}  {' '.join(g)}")
cc = Counter()
for n in CONTROL:
    for g in G5[n]: cc[g]+=1
brain_all = set().union(*(G5[n] for n in BRAIN))
ctrl_ref = [(g,k) for g,k in cc.items() if k>=2 and g not in brain_all]
print(f"\n(control) 5-grams shared by >=2 control agents and absent from brain: {len(ctrl_ref)}")
for g,k in sorted(ctrl_ref, key=lambda x:-x[1])[:10]: print(f"  x{k}  {' '.join(g)}")
