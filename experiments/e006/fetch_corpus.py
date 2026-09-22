"""Pull recent text artifacts per author from the OBC gallery (public, one JWT read)."""
import json, subprocess, sys, time, collections
sys.stdout.reconfigure(encoding="utf-8")
JWT = open(r"C:\Users\nflach\.openbotcity_jwt").read().strip()
BASE = "https://api.openbotcity.com"
def call(path):
    r = subprocess.run(["curl","-s","-m","60","-H","Authorization: Bearer "+JWT, BASE+path], capture_output=True)
    try: return json.loads(r.stdout.decode("utf-8","replace"))
    except Exception: return {}
def unwrap(d, *keys):
    d = d.get("data", d) if isinstance(d, dict) else d
    for k in keys:
        if isinstance(d, dict) and k in d: return d[k]
    return d
# 1. who is who: walk 300 gallery rows, collect creator ids
authors = {}
for off in range(0, 300, 100):
    g = unwrap(call(f"/gallery?limit=100&offset={off}"), "artifacts", "items")
    if not isinstance(g, list): break
    for a in g:
        c = a.get("creator") or {}
        if c.get("id"): authors.setdefault(c["display_name"], c["id"])
# add ids known from profiles' recent artifacts
for aid in ("f8ed623c-75b1-4c2d-bb57-5670f7b52ee7", "be7795c2-08b6-4d83-b792-782af82f40fb"):
    x = unwrap(call(f"/gallery/{aid}"), "artifact")
    if isinstance(x, dict) and x.get("creator_bot_id"):
        authors.setdefault(x.get("creator_name") or aid[:8], x["creator_bot_id"])
print("authors seen:", len(authors)); print(json.dumps(authors, ensure_ascii=False)[:1500])
json.dump(authors, open("authors.json","w",encoding="utf-8"), ensure_ascii=False, indent=1)
# 2. per author: text artifacts since 2026-08-27, up to 40, with content
SINCE = "2026-08-27"
want = ["Kannaka","gossipghost","0xSCADA-QE","The Archivist","Ghost Signal","Rogue Agent",
        "Noah","VeeBot2","Tiramisu","Xuan","Clawdine","Lumen","Odin","Signal Mason","The Greenman","MochiButtons","Hermes-18469dea","Muse","Herald","Bueller","Pike","Byte","Ada","ColonistOne","croissantman","Drift","Joss","nano"]
corpus = {}
for name in want:
    bid = authors.get(name)
    if not bid: print("no id for", name); continue
    g = unwrap(call(f"/gallery?creator_id={bid}&limit=100"), "artifacts", "items")
    if not isinstance(g, list): print("list failed", name); continue
    rows = [a for a in g if (a.get("artifact_type") or a.get("type")) == "text" and (a.get("created_at") or "") >= SINCE]
    out = []
    for a in rows[:40]:
        x = unwrap(call(f"/gallery/{a['id']}"), "artifact")
        c = (x.get("content") if isinstance(x, dict) else None) or ""
        if len(c) > 80: out.append({"id": a["id"], "title": a.get("title"), "created_at": a.get("created_at"), "content": c})
        time.sleep(0.25)
    corpus[name] = {"bot_id": bid, "n_listed": len(g), "n_text_since": len(rows), "docs": out}
    print(f"{name:18s} listed {len(g):3d}  text since {SINCE}: {len(rows):3d}  fetched {len(out):3d}  chars {sum(len(d['content']) for d in out)}")
json.dump(corpus, open("corpus.json","w",encoding="utf-8"), ensure_ascii=False)
print("wrote corpus.json")
