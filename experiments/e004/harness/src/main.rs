//! E-004 / E-007 harness. Four jobs:
//!
//!   selftest — the guards on synthetic streams, before any real data: a
//!              constant stream gives surprise exactly 0; a predictable stream
//!              is learned and adopted; pure noise is not adopted (it beats
//!              last-state by predicting near the mean, which is why the
//!              constant-mean baseline of Amendment 2 exists); both arms forget
//!              at least three quarters under the cap.
//!   prepare  — embed every event text and every facet once, in batches, into
//!              one cache, with E-001's encoder (`mxbai-embed-large`, 1024-d).
//!   train    — the predictor on days 1–20, scored on days 21–30 against the
//!              last-state baseline and the constant-mean baseline with paired
//!              bootstraps; the collapse guard; the adoption verdict. Stops the
//!              experiment if not adopted.
//!   run      — one seed, one arm: absorb days 21–30 in order with a dream per
//!              day under the cap, then the E-007 survival numbers, the E-004
//!              recall@10 numbers if probes are given, and Φ. One JSON per run.
//!
//! Everything that is a choice is on the command line and lands in the JSON,
//! so `report.py` says what ran rather than what was meant.
//!
//! Input: `events.jsonl`, one event per line in stream order:
//!   {"key": "<subject>#<seq>", "agent": "...", "day": 1..30, "text": "..."}
//! Labels (E-007): a JSON array of keys recalled later. Probes (E-004): a JSON
//! array of {"key": ..., "query": ...}, the query written from the settlement's
//! side. Every query must be in the cache (`prepare` embeds them when given).
mod mlp;

use kannaka_wave::novelty::{Drive, NoveltyDetector};
use kannaka_wave::store::VectorStore;
use kannaka_wave::{Encoder, Id, Retention, Substrate, Vector};
use mlp::{dist, mse, Mlp, Rng};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Deserialize, Clone)]
struct Event {
    key: String,
    agent: String,
    day: u32,
    text: String,
}

#[derive(Deserialize)]
struct Probe {
    key: String,
    query: String,
}

// ---- fixed choices (E-004 §The predictor), overridable on the command line ----
const K: usize = 8;
const HIDDEN: usize = 512;
const DIMS: usize = 1024;
const BOOTSTRAP: usize = 1000;
const COLLAPSE_FLOOR: f32 = 1.0 / 3.0;
const SURPRISE_CLIP: f32 = 3.0;
const BASE_IMPORTANCE: f32 = 0.5;

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == name)
        .and_then(|i| a.get(i + 1).cloned())
}

fn arg_or<T: std::str::FromStr>(name: &str, default: T) -> T {
    arg(name).and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn has(name: &str) -> bool {
    std::env::args().any(|x| x == name)
}

// ---- the vector cache -------------------------------------------------------

/// Text → unit vector, filled once by `prepare`. Every text the arms will
/// encode (events, their facets, probe queries) must be here; a miss is a
/// loud failure, never a silent zero vector.
struct Cache {
    dims: usize,
    map: HashMap<String, Vec<f32>>,
}

impl Cache {
    fn load(path: &Path) -> Cache {
        let mut b = Vec::new();
        std::fs::File::open(path)
            .unwrap_or_else(|e| panic!("cache {}: {e}", path.display()))
            .read_to_end(&mut b)
            .expect("read cache");
        let mut at = 0;
        let rd_u32 = |at: &mut usize| {
            let v = u32::from_le_bytes(b[*at..*at + 4].try_into().unwrap());
            *at += 4;
            v as usize
        };
        let dims = rd_u32(&mut at);
        let mut map = HashMap::new();
        while at < b.len() {
            let n = rd_u32(&mut at);
            let text = String::from_utf8(b[at..at + n].to_vec()).expect("utf-8");
            at += n;
            let mut v = Vec::with_capacity(dims);
            for _ in 0..dims {
                v.push(f32::from_le_bytes(b[at..at + 4].try_into().unwrap()));
                at += 4;
            }
            map.insert(text, v);
        }
        Cache { dims, map }
    }

    fn save(&self, path: &Path) {
        let mut f = std::fs::File::create(path).expect("create cache");
        f.write_all(&(self.dims as u32).to_le_bytes()).unwrap();
        for (t, v) in &self.map {
            f.write_all(&(t.len() as u32).to_le_bytes()).unwrap();
            f.write_all(t.as_bytes()).unwrap();
            for x in v {
                f.write_all(&x.to_le_bytes()).unwrap();
            }
        }
    }

    fn get(&self, text: &str) -> &[f32] {
        self.map.get(text).unwrap_or_else(|| {
            panic!(
                "not in cache: {:?}",
                text.chars().take(80).collect::<String>()
            )
        })
    }
}

impl Encoder for Cache {
    fn encode(&self, text: &str) -> Vector {
        Vector(self.get(text).to_vec())
    }
    fn dims(&self) -> usize {
        self.dims
    }
}

fn read_events(path: &Path) -> Vec<Event> {
    let f = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    std::io::BufReader::new(f)
        .lines()
        .map(|l| l.expect("line"))
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(&l).expect("event json"))
        .collect()
}

/// Every text the arms will ask the encoder for: the event, its facets, and
/// any probe queries.
fn texts_to_embed(events: &[Event], probes: &[Probe]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let mut push = |t: String| {
        if seen.insert(t.clone()) {
            out.push(t);
        }
    };
    for e in events {
        for f in kannaka_wave::facet::decompose(&e.text) {
            push(f);
        }
        push(e.text.clone());
    }
    for p in probes {
        push(p.query.clone());
    }
    out
}

// ---- prepare ----------------------------------------------------------------

fn prepare() {
    let events = read_events(&PathBuf::from(arg("--events").expect("--events")));
    let probes: Vec<Probe> = arg("--probes")
        .map(|p| {
            serde_json::from_str(&std::fs::read_to_string(p).expect("probes")).expect("probes json")
        })
        .unwrap_or_default();
    let out = PathBuf::from(arg("--cache").expect("--cache"));
    let dims: usize = arg_or("--dims", DIMS);
    let enc = kannaka_wave::encoder::OllamaEncoder::new(
        &arg("--host").unwrap_or_else(|| "127.0.0.1".into()),
        arg_or("--port", 11434u16),
        &arg("--model").unwrap_or_else(|| "mxbai-embed-large".into()),
        dims,
    )
    .with_timeout(std::time::Duration::from_secs(600));
    let mut cache = if out.exists() {
        Cache::load(&out)
    } else {
        Cache {
            dims,
            map: HashMap::new(),
        }
    };
    let todo: Vec<String> = texts_to_embed(&events, &probes)
        .into_iter()
        .filter(|t| !cache.map.contains_key(t))
        .collect();
    eprintln!(
        "[prepare] {} texts to embed ({} cached)",
        todo.len(),
        cache.map.len()
    );
    for (i, chunk) in todo.chunks(16).enumerate() {
        let refs: Vec<&str> = chunk.iter().map(String::as_str).collect();
        let vecs = match enc.try_encode_batch(&refs) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[prepare] batch {i} failed ({e}); one at a time");
                refs.iter()
                    .map(|t| {
                        enc.try_encode(t)
                            .unwrap_or_else(|e| panic!("encode {:?}: {e}", &t[..t.len().min(60)]))
                    })
                    .collect()
            }
        };
        for (t, v) in chunk.iter().zip(vecs) {
            cache.map.insert(t.clone(), v.0);
        }
        if i % 10 == 9 {
            cache.save(&out);
            eprintln!("[prepare] {} / {}", (i + 1) * 16, todo.len());
        }
    }
    cache.save(&out);
    eprintln!(
        "[prepare] cache holds {} vectors at {}",
        cache.map.len(),
        out.display()
    );
}

// ---- the predictor and its guards -------------------------------------------

#[derive(Serialize, Clone)]
struct Adoption {
    seed: u64,
    k: usize,
    hidden: usize,
    epochs: usize,
    lr: f32,
    train_samples: usize,
    heldout_samples: usize,
    heldout_mse_mlp: f32,
    heldout_mse_baseline: f32,
    /// A constant predictor: the mean of the training targets. Amendment 2:
    /// on a noisy stream it beats last-state without knowing anything, so the
    /// predictor must beat it too.
    heldout_mse_mean: f32,
    /// Paired bootstrap interval of (baseline − mlp) per held-out event; the
    /// predictor beats the baseline if `lo > 0`.
    diff_mean: f32,
    diff_lo: f32,
    diff_hi: f32,
    /// The same against the constant predictor.
    diff_vs_mean_lo: f32,
    diff_vs_mean_hi: f32,
    /// var(predictions) / var(targets) over held-out; void below 1/3.
    collapse_ratio: f32,
    beats_baseline: bool,
    beats_mean: bool,
    collapsed: bool,
    adopted: bool,
}

struct Trained {
    model: Mlp,
    adoption: Adoption,
}

#[allow(clippy::too_many_arguments)]
fn train_predictor(
    vectors: &[Vec<f32>],
    days: &[u32],
    train_days: (u32, u32),
    heldout_days: (u32, u32),
    seed: u64,
    k: usize,
    hidden: usize,
    epochs: usize,
    lr: f32,
) -> Trained {
    let dims = vectors[0].len();
    let in_range = |d: u32, r: (u32, u32)| d >= r.0 && d <= r.1;
    let mut model = Mlp::new(k, dims, hidden, seed);
    let mut rng = Rng::new(seed ^ 0xA5A5);
    let sample = |i: usize| -> (Vec<&[f32]>, &[f32]) {
        (
            (i - k..i).map(|j| vectors[j].as_slice()).collect(),
            vectors[i].as_slice(),
        )
    };
    let train: Vec<(Vec<&[f32]>, &[f32])> = (k..vectors.len())
        .filter(|&i| in_range(days[i], train_days))
        .map(sample)
        .collect();
    let heldout_idx: Vec<usize> = (k..vectors.len())
        .filter(|&i| in_range(days[i], heldout_days))
        .collect();
    assert!(!train.is_empty(), "no training samples");
    assert!(!heldout_idx.is_empty(), "no held-out samples");
    for ep in 0..epochs {
        let l = model.train_epoch(&train, 32, lr, &mut rng);
        if ep % 10 == 0 || ep + 1 == epochs {
            eprintln!("[train] seed {seed} epoch {ep} mse {l:.6}");
        }
    }
    // The constant predictor: mean of the training targets.
    let mut train_mean = vec![0.0f32; dims];
    for (_, t) in &train {
        for (m, x) in train_mean.iter_mut().zip(*t) {
            *m += x / train.len() as f32;
        }
    }
    // Held-out: paired per event.
    let mut diffs = Vec::with_capacity(heldout_idx.len());
    let mut diffs_mean = Vec::with_capacity(heldout_idx.len());
    let mut m_mlp = 0.0;
    let mut m_base = 0.0;
    let mut m_mean = 0.0;
    let mut preds: Vec<Vec<f32>> = Vec::with_capacity(heldout_idx.len());
    for &i in &heldout_idx {
        let (ctx, target) = sample(i);
        let p = model.predict(&ctx);
        let a = mse(&p, target);
        let b = mse(&vectors[i - 1], target);
        let c = mse(&train_mean, target);
        m_mlp += a;
        m_base += b;
        m_mean += c;
        diffs.push(b - a);
        diffs_mean.push(c - a);
        preds.push(p);
    }
    let n = heldout_idx.len() as f32;
    let (lo, hi, mean) = bootstrap_mean(&diffs, BOOTSTRAP, seed);
    let (mlo, mhi, _) = bootstrap_mean(&diffs_mean, BOOTSTRAP, seed ^ 0x77);
    let var_of = |vs: &[Vec<f32>]| -> f32 {
        let d = vs[0].len();
        let mut mean = vec![0.0f32; d];
        for v in vs {
            for (m, x) in mean.iter_mut().zip(v) {
                *m += x / vs.len() as f32;
            }
        }
        vs.iter()
            .map(|v| {
                v.iter()
                    .zip(&mean)
                    .map(|(x, m)| (x - m) * (x - m))
                    .sum::<f32>()
            })
            .sum::<f32>()
            / vs.len() as f32
    };
    let targets: Vec<Vec<f32>> = heldout_idx.iter().map(|&i| vectors[i].clone()).collect();
    let vt = var_of(&targets);
    let collapse_ratio = if vt > 0.0 { var_of(&preds) / vt } else { 0.0 };
    let beats = lo > 0.0;
    let beats_mean = mlo > 0.0;
    let collapsed = collapse_ratio < COLLAPSE_FLOOR;
    let adoption = Adoption {
        seed,
        k,
        hidden,
        epochs,
        lr,
        train_samples: train.len(),
        heldout_samples: heldout_idx.len(),
        heldout_mse_mlp: m_mlp / n,
        heldout_mse_baseline: m_base / n,
        heldout_mse_mean: m_mean / n,
        diff_mean: mean,
        diff_lo: lo,
        diff_hi: hi,
        diff_vs_mean_lo: mlo,
        diff_vs_mean_hi: mhi,
        collapse_ratio,
        beats_baseline: beats,
        beats_mean,
        collapsed,
        adopted: beats && beats_mean && !collapsed,
    };
    Trained { model, adoption }
}

/// Percentile bootstrap of the mean: (lo, hi, mean) at 95%.
fn bootstrap_mean(xs: &[f32], reps: usize, seed: u64) -> (f32, f32, f32) {
    let mut rng = Rng::new(seed ^ 0xB007);
    let n = xs.len();
    let mean = xs.iter().sum::<f32>() / n as f32;
    let mut means: Vec<f32> = (0..reps)
        .map(|_| {
            (0..n)
                .map(|_| xs[(rng.next_u64() % n as u64) as usize])
                .sum::<f32>()
                / n as f32
        })
        .collect();
    means.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (
        means[(reps as f32 * 0.025) as usize],
        means[(reps as f32 * 0.975) as usize],
        mean,
    )
}

// ---- the arms -----------------------------------------------------------------

#[derive(Serialize)]
struct RunOut {
    seed: u64,
    arm: String,
    cap: usize,
    heldout_days: (u32, u32),
    absorbed: usize,
    survivors: usize,
    forgot_frac: f32,
    forgot_three_quarters: bool,
    surprise: SurpriseStats,
    e007: Option<Survival>,
    e004: Option<Recall>,
    phi: serde_json::Value,
    predictor: Adoption,
}

#[derive(Serialize, Default)]
struct SurpriseStats {
    events: usize,
    nonzero: usize,
    mean_score: f32,
    max_score: f32,
    mean_importance: f32,
}

#[derive(Serialize)]
struct Survival {
    labels: usize,
    kept: usize,
    kept_frac: f32,
    precision: f32,
    /// kept-recalled by write-day, for the recency trap.
    by_day: Vec<(u32, usize, usize)>,
}

#[derive(Serialize)]
struct Recall {
    probes: usize,
    hits: usize,
    recall10: f32,
    survivors_load_bearing: usize,
    precision: f32,
}

/// Absorb the held-out days in order, one dream per day under the cap.
/// Arm S scales importance by `1 + surprise`, clipped at 3×; arm U is uniform.
/// Returns the store and the parent-id → event index map.
#[allow(clippy::too_many_arguments)]
fn run_arm(
    events: &[Event],
    cache: &dyn Encoder,
    vectors: &[Vec<f32>],
    heldout: (u32, u32),
    arm: &str,
    cap: usize,
    model: Option<&Mlp>,
    k: usize,
    stats: &mut SurpriseStats,
) -> (VectorStore, HashMap<Id, usize>) {
    let mut store = VectorStore::with_salt(cache.dims(), 7);
    let mut owner: HashMap<Id, usize> = HashMap::new();
    let policy = [Retention {
        class: String::new(),
        cap: Some(cap),
        ttl_days: None,
    }];
    let mut detector = NoveltyDetector::new(Drive::Error);
    let mut day_now: Option<u32> = None;
    let mut now: u64 = 1;
    let mut imp_total = 0.0;
    for (i, e) in events.iter().enumerate() {
        if e.day < heldout.0 || e.day > heldout.1 {
            continue;
        }
        if let Some(d) = day_now {
            if e.day != d {
                store.dream_at(&policy, now);
            }
        }
        day_now = Some(e.day);
        now += 1;
        let importance = match (arm, model) {
            ("S", Some(m)) if i >= k => {
                let ctx: Vec<&[f32]> = (i - k..i).map(|j| vectors[j].as_slice()).collect();
                let drive = dist(&m.predict(&ctx), &vectors[i]);
                let n = detector.observe(&e.agent, drive);
                stats.events += 1;
                if n.score > 0.0 {
                    stats.nonzero += 1;
                }
                stats.mean_score += n.score;
                stats.max_score = stats.max_score.max(n.score);
                BASE_IMPORTANCE * (1.0 + n.score).min(SURPRISE_CLIP)
            }
            ("S", Some(_)) => {
                stats.events += 1;
                BASE_IMPORTANCE
            }
            _ => BASE_IMPORTANCE,
        };
        imp_total += importance;
        let (pid, _) = store.absorb_experience_at(&e.text, importance, cache, now);
        owner.insert(pid, i);
    }
    store.dream_at(&policy, now + 1);
    if stats.events > 0 {
        stats.mean_score /= stats.events as f32;
    }
    let absorbed = owner.len().max(1);
    stats.mean_importance = imp_total / absorbed as f32;
    (store, owner)
}

fn phi_of(vecs: &[Vec<f32>], k: usize, parts: usize) -> serde_json::Value {
    // E-001's one Φ instrument, verbatim in method: k-NN cosine graph over
    // the survivors, spherical k-means partition (fixed seed), IIT Φ.
    let mut vecs: Vec<Vec<f32>> = vecs.to_vec();
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
        return serde_json::json!({"phi": 0.0, "n": n, "note": "too few survivors"});
    }
    let dot = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
    let mut conns: Vec<Vec<usize>> = Vec::with_capacity(n);
    for i in 0..n {
        let mut s: Vec<(f32, usize)> = (0..n)
            .filter(|&j| j != i)
            .map(|j| (dot(&vecs[i], &vecs[j]), j))
            .collect();
        s.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        conns.push(s.iter().take(k).map(|x| x.1).collect());
    }
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
        .map(|i| consciousness_core::iit::PhiNode {
            partition: label[i],
            connections: conns[i].clone(),
        })
        .collect();
    let r = consciousness_core::iit::compute_phi(&nodes);
    serde_json::json!({"phi": r.phi, "integration": r.integration, "differentiation": r.differentiation, "n": n, "k": k, "parts": parts})
}

fn score(
    store: &VectorStore,
    owner: &HashMap<Id, usize>,
    events: &[Event],
    cache: &Cache,
    labels: Option<&HashSet<String>>,
    probes: Option<&[Probe]>,
) -> (usize, Option<Survival>, Option<Recall>, Vec<Vec<f32>>) {
    let survivors: Vec<(Id, usize)> = store
        .rows()
        .iter()
        .filter(|r| r.parent.is_none())
        .map(|r| (r.id, owner[&r.id]))
        .collect();
    let surviving_keys: HashSet<&str> = survivors
        .iter()
        .map(|(_, i)| events[*i].key.as_str())
        .collect();
    let vecs: Vec<Vec<f32>> = survivors
        .iter()
        .map(|(_, i)| cache.get(&events[*i].text).to_vec())
        .collect();

    let e007 = labels.map(|labels| {
        let held: Vec<&Event> = owner.values().map(|&i| &events[i]).collect();
        let labelled: Vec<&Event> = held
            .iter()
            .copied()
            .filter(|e| labels.contains(&e.key))
            .collect();
        let kept = labelled
            .iter()
            .filter(|e| surviving_keys.contains(e.key.as_str()))
            .count();
        let mut by_day: HashMap<u32, (usize, usize)> = HashMap::new();
        for e in &labelled {
            let d = by_day.entry(e.day).or_default();
            d.0 += 1;
            if surviving_keys.contains(e.key.as_str()) {
                d.1 += 1;
            }
        }
        let mut by_day: Vec<(u32, usize, usize)> =
            by_day.into_iter().map(|(d, (n, k))| (d, n, k)).collect();
        by_day.sort();
        Survival {
            labels: labelled.len(),
            kept,
            kept_frac: if labelled.is_empty() {
                0.0
            } else {
                kept as f32 / labelled.len() as f32
            },
            precision: if survivors.is_empty() {
                0.0
            } else {
                kept as f32 / survivors.len() as f32
            },
            by_day,
        }
    });

    let e004 = probes.map(|probes| {
        let mut hits = 0;
        for p in probes {
            let q = Vector(cache.get(&p.query).to_vec());
            let got = store.recall(&q, 10);
            if got.iter().any(|r| {
                owner
                    .get(&r.id)
                    .map(|&i| events[i].key == p.key)
                    .unwrap_or(false)
            }) {
                hits += 1;
            }
        }
        let lb: HashSet<&str> = probes.iter().map(|p| p.key.as_str()).collect();
        let survivors_lb = surviving_keys.iter().filter(|k| lb.contains(*k)).count();
        Recall {
            probes: probes.len(),
            hits,
            recall10: if probes.is_empty() {
                0.0
            } else {
                hits as f32 / probes.len() as f32
            },
            survivors_load_bearing: survivors_lb,
            precision: if survivors.is_empty() {
                0.0
            } else {
                survivors_lb as f32 / survivors.len() as f32
            },
        }
    });
    (survivors.len(), e007, e004, vecs)
}

fn parse_days(s: &str) -> (u32, u32) {
    let (a, b) = s.split_once("..").expect("days as A..B");
    (a.parse().unwrap(), b.parse().unwrap())
}

fn train_or_run(run: bool) {
    let events = read_events(&PathBuf::from(arg("--events").expect("--events")));
    let cache = Cache::load(&PathBuf::from(arg("--cache").expect("--cache")));
    let vectors: Vec<Vec<f32>> = events.iter().map(|e| cache.get(&e.text).to_vec()).collect();
    let days: Vec<u32> = events.iter().map(|e| e.day).collect();
    let train_days = parse_days(&arg("--train-days").unwrap_or_else(|| "1..20".into()));
    let heldout = parse_days(&arg("--heldout-days").unwrap_or_else(|| "21..30".into()));
    let seed: u64 = arg_or("--seed", 1);
    let k: usize = arg_or("--k", K);
    let hidden: usize = arg_or("--hidden", HIDDEN);
    let epochs: usize = arg_or("--epochs", 30);
    let lr: f32 = arg_or("--lr", 1e-3);
    let arm = arg("--arm").unwrap_or_else(|| "U".into());

    let trained = if run && arm == "U" {
        None
    } else {
        Some(train_predictor(
            &vectors, &days, train_days, heldout, seed, k, hidden, epochs, lr,
        ))
    };
    if !run {
        println!(
            "{}",
            serde_json::to_string_pretty(&trained.unwrap().adoption).unwrap()
        );
        return;
    }
    if let Some(t) = &trained {
        if !t.adoption.adopted && !has("--force") {
            eprintln!(
                "[run] predictor not adopted (beats last-state: {}, beats mean: {}, collapsed: {}); E-004 stops here with that finding. --force overrides for diagnostics only.",
                t.adoption.beats_baseline, t.adoption.beats_mean, t.adoption.collapsed
            );
            println!("{}", serde_json::to_string_pretty(&t.adoption).unwrap());
            std::process::exit(3);
        }
    }
    let cap: usize = arg_or("--cap", 0);
    assert!(cap > 0, "--cap is required: the cap that forces forgetting");
    let labels: Option<HashSet<String>> = arg("--labels").map(|p| {
        serde_json::from_str::<Vec<String>>(&std::fs::read_to_string(p).expect("labels"))
            .expect("labels json")
            .into_iter()
            .collect()
    });
    let probes: Option<Vec<Probe>> = arg("--probes").map(|p| {
        serde_json::from_str(&std::fs::read_to_string(p).expect("probes")).expect("probes json")
    });

    let mut stats = SurpriseStats::default();
    let (store, owner) = run_arm(
        &events,
        &cache,
        &vectors,
        heldout,
        &arm,
        cap,
        trained.as_ref().map(|t| &t.model),
        k,
        &mut stats,
    );
    let (survivors, e007, e004, vecs) = score(
        &store,
        &owner,
        &events,
        &cache,
        labels.as_ref(),
        probes.as_deref(),
    );
    let absorbed = owner.len();
    let forgot = 1.0 - survivors as f32 / absorbed.max(1) as f32;
    let out = RunOut {
        seed,
        arm: arm.clone(),
        cap,
        heldout_days: heldout,
        absorbed,
        survivors,
        forgot_frac: forgot,
        forgot_three_quarters: forgot >= 0.75,
        surprise: stats,
        e007,
        e004,
        phi: phi_of(&vecs, 8, 8),
        predictor: trained.map(|t| t.adoption).unwrap_or_else(|| Adoption {
            seed,
            k,
            hidden,
            epochs,
            lr,
            train_samples: 0,
            heldout_samples: 0,
            heldout_mse_mlp: 0.0,
            heldout_mse_baseline: 0.0,
            heldout_mse_mean: 0.0,
            diff_mean: 0.0,
            diff_lo: 0.0,
            diff_hi: 0.0,
            diff_vs_mean_lo: 0.0,
            diff_vs_mean_hi: 0.0,
            collapse_ratio: 0.0,
            beats_baseline: false,
            beats_mean: false,
            collapsed: false,
            adopted: false,
        }),
    };
    let json = serde_json::to_string_pretty(&out).unwrap();
    match arg("--out") {
        Some(p) => std::fs::write(&p, &json).expect("write out"),
        None => println!("{json}"),
    }
}

// ---- selftest -----------------------------------------------------------------

fn unit(mut v: Vec<f32>) -> Vec<f32> {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-9);
    for x in v.iter_mut() {
        *x /= n;
    }
    v
}

fn selftest() {
    let dims = 16;
    let k = 4;
    let mut fails = 0;
    let mut check = |name: &str, ok: bool, detail: String| {
        println!("{} {name}: {detail}", if ok { "ok  " } else { "FAIL" });
        if !ok {
            fails += 1;
        }
    };

    // 1. A constant drive gives surprise exactly 0, for every context.
    let mut det = NoveltyDetector::new(Drive::Error);
    let mut worst = 0.0f32;
    for i in 0..300 {
        let n = det.observe(if i % 2 == 0 { "a" } else { "b" }, 0.7);
        worst = worst.max(n.score.abs());
    }
    check(
        "constant stream gives surprise 0",
        worst == 0.0,
        format!("max |score| = {worst}"),
    );

    // 2. A predictable stream (a fixed rotation of the last state) is learned:
    //    the predictor beats last-state with the interval excluding zero, and
    //    does not collapse.
    let mut rng = Rng::new(11);
    let mut z = unit((0..dims).map(|_| rng.normal()).collect());
    let rotate = |v: &[f32]| -> Vec<f32> {
        let mut r: Vec<f32> = v[1..].to_vec();
        r.push(-v[0]);
        unit(
            r.iter()
                .enumerate()
                .map(|(i, x)| x + 0.05 * ((i as f32) * 0.7).sin())
                .collect(),
        )
    };
    let mut vectors = Vec::new();
    let mut days = Vec::new();
    for t in 0..600 {
        z = rotate(&z);
        vectors.push(z.clone());
        days.push((t / 20) as u32 + 1);
    }
    let t = train_predictor(&vectors, &days, (1, 20), (21, 30), 1, k, 32, 60, 3e-3);
    check(
        "predictable stream: adopted",
        t.adoption.adopted,
        format!(
            "mlp {:.5} vs last-state {:.5} vs mean {:.5}, diff [{:.5}, {:.5}], vs mean [{:.5}, {:.5}], collapse ratio {:.2}",
            t.adoption.heldout_mse_mlp, t.adoption.heldout_mse_baseline, t.adoption.heldout_mse_mean, t.adoption.diff_lo, t.adoption.diff_hi,
            t.adoption.diff_vs_mean_lo, t.adoption.diff_vs_mean_hi, t.adoption.collapse_ratio
        ),
    );

    // 3. Pure noise. Predicting near the mean beats last-state (about 1/D
    //    against 2/D) while knowing nothing, and the first run of this guard
    //    showed the collapse floor of 1/3 does not catch it (ratio 0.58). So
    //    Amendment 2: the predictor must also beat the constant mean
    //    predictor. Recorded, not tuned: the floor stays at 1/3.
    let mut rng = Rng::new(12);
    let noise: Vec<Vec<f32>> = (0..600)
        .map(|_| unit((0..dims).map(|_| rng.normal()).collect()))
        .collect();
    let tn = train_predictor(&noise, &days, (1, 20), (21, 30), 1, k, 32, 60, 3e-3);
    check(
        "noise: not adopted",
        !tn.adoption.adopted,
        format!(
            "beats last-state: {} (mlp {:.5} vs {:.5}); beats mean: {} (vs {:.5}); collapse ratio {:.2} (floor {:.2})",
            tn.adoption.beats_baseline, tn.adoption.heldout_mse_mlp, tn.adoption.heldout_mse_baseline,
            tn.adoption.beats_mean, tn.adoption.heldout_mse_mean, tn.adoption.collapse_ratio, COLLAPSE_FLOOR
        ),
    );

    // 4. Both arms forget at least three quarters under the cap, and arm S's
    //    importances are not uniform.
    let events: Vec<Event> = (0..600)
        .map(|i| Event {
            key: format!("k{i}"),
            agent: if i % 3 == 0 { "x".into() } else { "y".into() },
            day: days[i],
            text: format!("synthetic event {i}"),
        })
        .collect();
    let cache = Cache {
        dims,
        map: events
            .iter()
            .enumerate()
            .map(|(i, e)| (e.text.clone(), vectors[i].clone()))
            .collect(),
    };
    let held: usize = events.iter().filter(|e| e.day >= 21).count();
    let cap = held / 5;
    let mut su = SurpriseStats::default();
    let (store_u, owner_u) = run_arm(
        &events,
        &cache,
        &vectors,
        (21, 30),
        "U",
        cap,
        None,
        k,
        &mut su,
    );
    let mut ss = SurpriseStats::default();
    let (store_s, owner_s) = run_arm(
        &events,
        &cache,
        &vectors,
        (21, 30),
        "S",
        cap,
        Some(&t.model),
        k,
        &mut ss,
    );
    let surv = |s: &VectorStore| s.rows().iter().filter(|r| r.parent.is_none()).count();
    let fu = 1.0 - surv(&store_u) as f32 / owner_u.len() as f32;
    let fs = 1.0 - surv(&store_s) as f32 / owner_s.len() as f32;
    check(
        "both arms forgot >= 3/4",
        fu >= 0.75 && fs >= 0.75,
        format!("U forgot {fu:.2}, S forgot {fs:.2} of {held} (cap {cap})"),
    );
    check(
        "arm S importances vary with surprise",
        ss.nonzero > 0 && ss.mean_importance != BASE_IMPORTANCE,
        format!(
            "{} of {} events surprising, mean importance {:.3} (U: {:.3})",
            ss.nonzero, ss.events, ss.mean_importance, su.mean_importance
        ),
    );
    let labels: HashSet<String> = events
        .iter()
        .filter(|e| e.day >= 21)
        .step_by(7)
        .map(|e| e.key.clone())
        .collect();
    let (_, e7, _, vecs) = score(&store_u, &owner_u, &events, &cache, Some(&labels), None);
    let e7 = e7.unwrap();
    let phi = phi_of(&vecs, 8, 8);
    check(
        "scoring runs end to end",
        e7.labels > 0 && phi["phi"].is_number(),
        format!("labels {}, kept {}, phi {}", e7.labels, e7.kept, phi["phi"]),
    );

    if fails > 0 {
        eprintln!("{fails} guard(s) failed");
        std::process::exit(1);
    }
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("selftest") => selftest(),
        Some("prepare") => prepare(),
        Some("train") => train_or_run(false),
        Some("run") => train_or_run(true),
        _ => {
            eprintln!(
                "e004-harness selftest\n\
                 e004-harness prepare --events events.jsonl --cache vectors.bin [--probes probes.json] [--host H --port P --model M --dims D]\n\
                 e004-harness train --events E --cache C [--seed 1] [--train-days 1..20] [--heldout-days 21..30] [--epochs 30] [--lr 1e-3]\n\
                 e004-harness run --events E --cache C --arm U|S --seed S --cap N [--labels labels.json] [--probes probes.json] [--out run.json]"
            );
            std::process::exit(2);
        }
    }
}
