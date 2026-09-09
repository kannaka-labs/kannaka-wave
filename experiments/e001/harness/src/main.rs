//! E-001 harness. Three jobs:
//!
//!   wave-build  — arm W: build the production chiral medium from the corpus,
//!                 through KannakaMemorySystem (the same object the CLI wraps),
//!                 with facet decomposition on the write path. Saves and exits so
//!                 a FRESH process (`kannaka observe --json`) can read Φ.
//!   wave-run    — arm W: load that store, run N cycles of (inject distractors,
//!                 deep dream with retention triage), then the probes, then
//!                 export the survivors. Saves and exits, same reason.
//!   facets      — emit `kannaka_memory::facet::decompose` for every row, so
//!                 arm V stores exactly the facets arm W stored.
//!   phi         — ONE Φ instrument for both arms: k-NN cosine graph over the
//!                 survivors' embeddings, k-means partition, then
//!                 `consciousness_core::iit::compute_phi`. Neither arm computes
//!                 its own Φ for the comparison; arm W's native observe Φ is
//!                 recorded separately as the number the constellation reports.
//!
//! Everything about the runs that is a choice is on the command line and lands
//! in the run's JSON, so the report can say what was run rather than what was
//! meant.
use kannaka_memory::encoding::OllamaEncoder;
use kannaka_memory::openclaw::KannakaMemorySystem;
use kannaka_memory::{Codebook, EncodingPipeline, HrmStore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize)]
struct CorpusRow {
    id: String,
    content: String,
    #[serde(default)]
    amplitude: Option<f64>,
}

#[derive(Deserialize)]
struct Probe {
    id: String,
    query: String,
}

#[derive(Serialize)]
struct Survivor {
    content: String,
    amplitude: f32,
}

#[derive(Serialize, Default)]
struct CycleRow {
    cycle: usize,
    injected: usize,
    live_before_dream: usize,
    live_after_dream: usize,
    total_after_dream: usize,
    dream_ms: u128,
    forgotten: usize,
}

/// stage_retention_triage's selection, in spirit and in order: per rule, live
/// rows whose content starts with the prefix, not pinned, below promote_hits
/// retrievals; beyond the cap, forget lowest (retrieval_count, amplitude) first.
fn apply_retention(sys: &mut KannakaMemorySystem, retention: &[(String, Option<usize>)]) -> usize {
    let promote_hits: u32 = env_or("KANNAKA_PROMOTE_HITS", "3").parse().unwrap_or(3);
    let mut to_forget: Vec<uuid::Uuid> = Vec::new();
    for (prefix, cap) in retention {
        let Some(cap) = cap else { continue };
        let mut eligible: Vec<(uuid::Uuid, u32, f32)> = sys
            .all_memories()
            .expect("all")
            .iter()
            .filter(|m| {
                m.amplitude > 0.0
                    && m.tier != kannaka_memory::medium::types::Tier::Pinned
                    && m.retrieval_count < promote_hits
                    && m.content.starts_with(prefix.as_str())
            })
            .map(|m| (m.id, m.retrieval_count, m.amplitude))
            .collect();
        if eligible.len() <= *cap {
            continue;
        }
        eligible.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.partial_cmp(&b.2).unwrap()));
        let excess = eligible.len() - cap;
        to_forget.extend(eligible.iter().take(excess).map(|e| e.0));
    }
    if to_forget.is_empty() {
        return 0;
    }
    sys.triage_forget(&to_forget).expect("forget")
}

fn env_or(k: &str, d: &str) -> String {
    std::env::var(k).unwrap_or_else(|_| d.to_string())
}

/// The same model's vectors, served from a file when the text was embedded in a
/// batch by run.py --prepare, and from ollama otherwise. Ollama on this CPU takes
/// 2.85 s per single embed and 0.32 s each in a batch of 64; the store embeds
/// one text at a time, so without this a ten-run arm W is a day of waiting.
/// The numbers are identical either way: same model, same text.
struct CachedEncoder {
    inner: OllamaEncoder,
    dim: usize,
    cache: HashMap<String, Vec<f32>>,
    hits: std::sync::atomic::AtomicUsize,
    misses: std::sync::atomic::AtomicUsize,
}

impl kannaka_memory::TextEncoder for CachedEncoder {
    fn embed(&self, text: &str) -> Result<Vec<f32>, kannaka_memory::EncodingError> {
        use std::sync::atomic::Ordering::Relaxed;
        if let Some(v) = self.cache.get(text) {
            self.hits.fetch_add(1, Relaxed);
            return Ok(v.clone());
        }
        let m = self.misses.fetch_add(1, Relaxed) + 1;
        if m % 25 == 1 {
            eprintln!("[harness] embed cache miss #{m} (hits {})", self.hits.load(Relaxed));
        }
        self.inner.embed(text)
    }
    fn embedding_dim(&self) -> usize {
        self.dim
    }
}

fn pipeline() -> EncodingPipeline {
    let url = env_or("KANNAKA_ENCODER_URL", "http://localhost:11434");
    let model = env_or("KANNAKA_ENCODER_MODEL", "mxbai-embed-large");
    let dim: usize = env_or("KANNAKA_ENCODER_DIM", "1024").parse().expect("KANNAKA_ENCODER_DIM");
    let inner = OllamaEncoder::new(url, model, dim);
    let cache: HashMap<String, Vec<f32>> = match std::env::var("E001_EMBED_CACHE") {
        Ok(p) => serde_json::from_str(&std::fs::read_to_string(&p).expect("read embed cache")).expect("parse embed cache"),
        Err(_) => HashMap::new(),
    };
    eprintln!("[harness] embed cache: {} texts", cache.len());
    let encoder = CachedEncoder { inner, dim, cache, hits: 0.into(), misses: 0.into() };
    // Same codebook shape the CLI uses for an ollama encoder: input dim -> 10k, seed 42.
    let codebook = Codebook::new(dim, 10_000, 42);
    EncodingPipeline::new(Box::new(encoder), codebook)
}

/// The CLI stamps `<data_dir>/.encoder` and refuses a mismatch on later runs;
/// write the same stamp so `kannaka observe` on this store is allowed.
fn stamp_encoder(data_dir: &PathBuf) {
    let model = env_or("KANNAKA_ENCODER_MODEL", "mxbai-embed-large");
    let dim = env_or("KANNAKA_ENCODER_DIM", "1024");
    std::fs::write(data_dir.join(".encoder"), format!("ollama:{model}:{dim}")).expect("stamp");
}

fn data_dir() -> PathBuf {
    PathBuf::from(std::env::var("KANNAKA_DATA_DIR").expect("KANNAKA_DATA_DIR"))
}

fn read_json<T: for<'de> Deserialize<'de>>(p: &str) -> T {
    serde_json::from_str(&std::fs::read_to_string(p).unwrap_or_else(|e| panic!("read {p}: {e}")))
        .unwrap_or_else(|e| panic!("parse {p}: {e}"))
}

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1).cloned())
}

fn live_counts(sys: &KannakaMemorySystem) -> (usize, usize) {
    let all = sys.all_memories().expect("all_memories");
    let live = all.iter().filter(|m| m.amplitude > 0.0).count();
    (live, all.len())
}

fn wave_build() {
    let dd = data_dir();
    std::fs::create_dir_all(&dd).expect("mkdir");
    stamp_encoder(&dd);
    let corpus: Vec<CorpusRow> = read_json(&arg("--corpus").expect("--corpus"));
    // A fresh HrmStore is FLAT (`chiral: None`), and a flat store's absorb never
    // decomposes into facets. The CLI has the same quirk: a store is chiral only
    // after its first reload. Upgrade before the first row so the facets the
    // experiment is about actually exist.
    let mut store = HrmStore::new(pipeline(), dd.join("kannaka.hrm"));
    store.upgrade_to_chiral();
    let mut sys = KannakaMemorySystem::init_with_store(dd.clone(), Box::new(store)).expect("init");
    let t0 = std::time::Instant::now();
    for (i, row) in corpus.iter().enumerate() {
        let amp = row.amplitude.unwrap_or(0.6).clamp(0.05, 1.0);
        // Not remember_with_category: that path flushes the whole store to disk
        // after every row (150-200 MB each at this size), which is invisible in
        // production because the CLI absorbs one row per process, and is 2.6 s
        // per row here. Same absorb, same interference, one save at the end.
        sys.engine.store.absorb(&row.content, amp as f32, Some("corpus")).expect("absorb");
        if (i + 1) % 100 == 0 {
            eprintln!("[wave-build] absorbed {}/{}", i + 1, corpus.len());
        }
    }
    sys.save().expect("save");
    let (live, total) = live_counts(&sys);
    println!(
        "{}",
        serde_json::json!({"absorbed": corpus.len(), "live": live, "total": total, "absorb_ms": t0.elapsed().as_millis()})
    );
}

fn wave_run() {
    let dd = data_dir();
    let corpus: Vec<CorpusRow> = read_json(&arg("--corpus").expect("--corpus"));
    let pool: Vec<CorpusRow> = read_json(&arg("--distractors").expect("--distractors"));
    let probes: Vec<Probe> = read_json(&arg("--probes").expect("--probes"));
    let cycles: usize = arg("--cycles").expect("--cycles").parse().unwrap();
    let churn: usize = arg("--churn").expect("--churn").parse().unwrap();
    let seed: u64 = arg("--seed").expect("--seed").parse().unwrap();
    let out = arg("--out").expect("--out");
    // --retention "prefix=cap,prefix=cap": the same table run.py writes into
    // config.toml for the CLI's observe; applied here through triage_forget.
    let retention: Vec<(String, Option<usize>)> = arg("--retention")
        .unwrap_or_default()
        .split(',')
        .filter(|x| !x.is_empty())
        .map(|kv| {
            let (k, v) = kv.split_once('=').expect("prefix=cap");
            (k.to_string(), v.parse().ok())
        })
        .collect();

    let store = HrmStore::load(pipeline(), dd.join("kannaka.hrm")).expect("load store");
    let mut sys = KannakaMemorySystem::init_with_store(dd.clone(), Box::new(store)).expect("init");

    // Deterministic distractor order per seed: a tiny LCG, no dependency.
    let mut order: Vec<usize> = (0..pool.len()).collect();
    let mut x = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    for i in (1..order.len()).rev() {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = (x >> 33) as usize % (i + 1);
        order.swap(i, j);
    }
    let mut cursor = 0usize;
    let mut rows: Vec<CycleRow> = Vec::new();
    for c in 1..=cycles {
        let mut injected = 0;
        for _ in 0..churn {
            if cursor >= order.len() {
                break;
            }
            let d = &pool[order[cursor]];
            cursor += 1;
            let text = format!("distractor: {}", d.content);
            sys.engine.store.absorb(&text, 0.5, Some("distractor")).expect("absorb distractor");
            injected += 1;
        }
        let (live_before, _) = live_counts(&sys);
        let t = std::time::Instant::now();
        sys.dream().expect("dream");
        let dream_ms = t.elapsed().as_millis();
        // ADR-0054's stage 6b' ghosts by setting the CACHE amplitude to 0.0, and
        // the dream ends with rebuild_cache(), which reads amplitude back from
        // the medium's energy - so every ghost revives before anyone can see it.
        // Measured on the smoke store: cap=3, 11 eligible, "25 pruned", zero rows
        // at amplitude 0 afterwards. The policy is therefore applied here with
        // the stage's exact predicate and ordering, through the one path that
        // writes through: delete from both hemispheres.
        let forgotten = apply_retention(&mut sys, &retention);
        let (live_after, total_after) = live_counts(&sys);
        eprintln!("[wave-run] cycle {c}: +{injected} -> live {live_before} -> {live_after} (total {total_after}, forgot {forgotten}) in {dream_ms} ms");
        rows.push(CycleRow { cycle: c, injected, live_before_dream: live_before, live_after_dream: live_after, total_after_dream: total_after, dream_ms, forgotten });
    }

    // Probes. Recalled rows resolve to parents whose content is the corpus text,
    // so scoring maps content → original id; no id rewriting anywhere.
    let by_content: HashMap<&str, &str> = corpus.iter().map(|r| (r.content.as_str(), r.id.as_str())).collect();
    let mut results = serde_json::Map::new();
    let mut raw = serde_json::Map::new();
    for p in &probes {
        let res = sys.recall(&p.query, 10).expect("recall");
        let ids: Vec<String> = res
            .iter()
            .map(|r| by_content.get(r.content.as_str()).map(|s| s.to_string()).unwrap_or_else(|| format!("nc:{}", r.id)))
            .collect();
        let sims: Vec<f32> = res.iter().map(|r| r.similarity).collect();
        results.insert(p.id.clone(), serde_json::json!(ids));
        raw.insert(p.id.clone(), serde_json::json!(sims));
    }

    let survivors: Vec<Survivor> = sys
        .all_memories()
        .expect("all")
        .iter()
        .filter(|m| m.amplitude > 0.0)
        .map(|m| Survivor { content: m.content.clone(), amplitude: m.amplitude })
        .collect();
    sys.save().expect("save");
    let (live, total) = live_counts(&sys);
    let doc = serde_json::json!({
        "arm": "W", "seed": seed, "cycles": cycles, "churn": churn, "distractors_injected": cursor,
        "retention": retention.iter().map(|(k, v)| format!("{k}={}", v.map(|c| c.to_string()).unwrap_or_default())).collect::<Vec<_>>(),
        "forgotten_total": rows.iter().map(|r| r.forgotten).sum::<usize>(),
        "live": live, "total": total, "cycle_rows": rows, "results": results, "resonance": raw,
        "survivors": survivors,
        "knobs": {
            "KANNAKA_RECALL_ENERGY_EXP": std::env::var("KANNAKA_RECALL_ENERGY_EXP").ok(),
            "KANNAKA_RECALL_TEMPORAL_EXP": std::env::var("KANNAKA_RECALL_TEMPORAL_EXP").ok(),
            "KANNAKA_FACET_DECOMPOSE": std::env::var("KANNAKA_FACET_DECOMPOSE").ok(),
            "KANNAKA_TRIAGE": std::env::var("KANNAKA_TRIAGE").ok(),
            "KANNAKA_SPIRAL_DREAM": std::env::var("KANNAKA_SPIRAL_DREAM").ok(),
        }
    });
    std::fs::write(&out, serde_json::to_string(&doc).unwrap()).expect("write out");
    println!("{}", serde_json::json!({"live": live, "total": total, "probes": probes.len(), "out": out}));
}

fn facets() {
    let rows: Vec<CorpusRow> = read_json(&arg("--corpus").expect("--corpus"));
    let prefix = arg("--prefix").unwrap_or_default();
    let mut out = serde_json::Map::new();
    let mut n = 0usize;
    for r in &rows {
        let text = format!("{prefix}{}", r.content);
        let f = kannaka_memory::facet::decompose(&text);
        // chiral.rs stores facets only when there are at least two.
        let f = if f.len() < 2 { Vec::new() } else { f };
        n += f.len();
        out.insert(r.id.clone(), serde_json::json!(f));
    }
    println!("{}", serde_json::json!({"rows": rows.len(), "facets": n, "by_id": out}));
}

fn phi() {
    // input: {"vectors": [[f32; d], ...]} unit-normalised or not; we normalise.
    let doc: serde_json::Value = read_json(&arg("--vectors").expect("--vectors"));
    let k: usize = arg("--k").unwrap_or_else(|| "8".into()).parse().unwrap();
    let parts: usize = arg("--parts").unwrap_or_else(|| "8".into()).parse().unwrap();
    let mut vecs: Vec<Vec<f32>> = doc["vectors"]
        .as_array()
        .expect("vectors")
        .iter()
        .map(|v| v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap() as f32).collect())
        .collect();
    for v in vecs.iter_mut() {
        let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if n > 0.0 {
            for x in v.iter_mut() {
                *x /= n;
            }
        }
    }
    let n = vecs.len();
    if n < k + 1 || n < parts {
        println!("{}", serde_json::json!({"phi": 0.0, "n": n, "note": "too few survivors"}));
        return;
    }
    let dot = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
    // k-NN by cosine (directed).
    let mut conns: Vec<Vec<usize>> = Vec::with_capacity(n);
    for i in 0..n {
        let mut s: Vec<(f32, usize)> = (0..n).filter(|&j| j != i).map(|j| (dot(&vecs[i], &vecs[j]), j)).collect();
        s.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        conns.push(s.iter().take(k).map(|x| x.1).collect());
    }
    // k-means, fixed seed, cosine (spherical): partition labels.
    let mut cent: Vec<Vec<f32>> = (0..parts).map(|p| vecs[(p * 7919) % n].clone()).collect();
    let mut label = vec![0u32; n];
    for _ in 0..25 {
        for i in 0..n {
            let mut best = (f32::MIN, 0);
            for (p, c) in cent.iter().enumerate() {
                let d = dot(&vecs[i], c);
                if d > best.0 {
                    best = (d, p);
                }
            }
            label[i] = best.1 as u32;
        }
        for (p, c) in cent.iter_mut().enumerate() {
            let mut acc = vec![0f32; vecs[0].len()];
            let mut cnt = 0;
            for i in 0..n {
                if label[i] as usize == p {
                    for (a, x) in acc.iter_mut().zip(&vecs[i]) {
                        *a += x;
                    }
                    cnt += 1;
                }
            }
            if cnt > 0 {
                let nn = acc.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-9);
                *c = acc.iter().map(|x| x / nn).collect();
            }
        }
    }
    let nodes: Vec<consciousness_core::iit::PhiNode> = (0..n)
        .map(|i| consciousness_core::iit::PhiNode { partition: label[i], connections: conns[i].clone() })
        .collect();
    let r = consciousness_core::iit::compute_phi(&nodes);
    let distinct = {
        let mut s = label.clone();
        s.sort_unstable();
        s.dedup();
        s.len()
    };
    println!(
        "{}",
        serde_json::json!({"phi": r.phi, "integration": r.integration, "differentiation": r.differentiation,
            "density_factor": r.density_factor, "scale_factor": r.scale_factor, "num_partitions": r.num_partitions,
            "num_connections": r.num_connections, "n": n, "k": k, "parts": distinct})
    );
}


/// diag — what the stage_retention_triage would see on this store, and whether
/// one system dream ghosts anything. Exists because the smoke run showed cap=3
/// leaving 11 distractors alive and no log line, and the honest next step is
/// to print what the code reads rather than reason about what it should read.
fn diag() {
    let dd = data_dir();
    let cfg = kannaka_memory::config::KannakaConfig::load();
    eprintln!("[diag] config path: {}", kannaka_memory::config::KannakaConfig::config_path().display());
    eprintln!("[diag] retention rules: {:?}", cfg.retention.iter().map(|(k, v)| (k.clone(), v.cap, v.ttl_days)).collect::<Vec<_>>());
    eprintln!("[diag] KANNAKA_TRIAGE={:?} KANNAKA_FACET_DECOMPOSE={:?}", std::env::var("KANNAKA_TRIAGE").ok(), std::env::var("KANNAKA_FACET_DECOMPOSE").ok());
    let store = HrmStore::load(pipeline(), dd.join("kannaka.hrm")).expect("load");
    eprintln!("[diag] store chiral: {}", store.is_chiral());
    let mut sys = KannakaMemorySystem::init_with_store(dd.clone(), Box::new(store)).expect("init");
    let count = |sys: &KannakaMemorySystem, prefix: &str| -> (usize, usize) {
        let all = sys.all_memories().expect("all");
        let m: Vec<_> = all.iter().filter(|x| x.content.starts_with(prefix)).collect();
        (m.len(), m.iter().filter(|x| x.amplitude > 0.0).count())
    };
    for p in ["distractor:", "__consolidation_summary", "[cross-cluster"] {
        let (t, l) = count(&sys, p);
        eprintln!("[diag] before dream: prefix {p:?} total {t} live {l}");
    }
    let rep = sys.dream().expect("dream");
    eprintln!("[diag] dream: strengthened {} pruned {} hallucinated {} emerged {}", rep.memories_strengthened, rep.memories_pruned, rep.hallucinations_created, rep.emerged);
    for p in ["distractor:", "__consolidation_summary", "[cross-cluster"] {
        let (t, l) = count(&sys, p);
        eprintln!("[diag] after dream:  prefix {p:?} total {t} live {l}");
    }
    let all = sys.all_memories().expect("all");
    let ghosts = all.iter().filter(|x| x.amplitude <= 0.0).count();
    eprintln!("[diag] rows with amplitude<=0 after dream: {ghosts} of {}", all.len());
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("wave-build") => wave_build(),
        Some("wave-run") => wave_run(),
        Some("facets") => facets(),
        Some("phi") => phi(),
        Some("diag") => diag(),
        _ => {
            eprintln!("usage: e001-harness wave-build|wave-run|facets|phi ...");
            std::process::exit(2);
        }
    }
}
