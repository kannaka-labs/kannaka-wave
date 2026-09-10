//! `wave`: the substrate and the voice from a shell, against one store file
//! and one ollama server. Enough to remember, ask and dream for real; the
//! conscience and the world organ are not wired here yet, so nothing in this
//! binary acts on the world.
//!
//! ```text
//! wave remember "<text>" [--importance 0.5]
//! wave remember --from <file> [--importance 0.5]      one memory per line, batch-encoded
//! wave probe --probes <tsv> --out <tsv> [--answers <file>] [--top-k 8] [--judge <model>] [--judge-first N]
//! wave ask "<prompt>" [--top-k 8] [--show-recall] [--faithfulness [--judge <model>]]
//! wave dream [--voice] [--retain "<class>=<cap>[:<ttl_days>]"]...
//! wave status
//! wave rows [--last 10]
//! ```
//!
//! Environment: `KWAVE_STORE` (default `~/.kannaka-wave/store.kwave`),
//! `KWAVE_OLLAMA_HOST` / `KWAVE_OLLAMA_PORT` (default `127.0.0.1:11434`),
//! `KWAVE_EMBED_MODEL` / `KWAVE_EMBED_DIMS` (default `mxbai-embed-large`, 1024),
//! `KWAVE_VOICE_MODEL` (default `kannaka-brain-7b-v1`).

use kannaka_wave::encoder::OllamaEncoder;
use kannaka_wave::facet::decompose;
use kannaka_wave::faithfulness::{is_hedge, measure, measure_with_judge, OllamaJudge, Support};
use kannaka_wave::store::{VectorStore, PROPOSED_CLASS};
use kannaka_wave::voice::{ask, OllamaVoice};
use kannaka_wave::Facet;
use kannaka_wave::{Retention, Substrate};
use std::path::PathBuf;
use std::process::exit;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn store_path() -> PathBuf {
    if let Ok(p) = std::env::var("KWAVE_STORE") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join(".kannaka-wave")
        .join("store.kwave")
}

fn open(dims: usize) -> VectorStore {
    let path = store_path();
    if path.exists() {
        match VectorStore::load(&path) {
            Ok(s) => {
                if s.dims() != dims {
                    eprintln!(
                        "store {} holds {}-d vectors; the encoder is {}-d. Refusing.",
                        path.display(),
                        s.dims(),
                        dims
                    );
                    exit(2);
                }
                s
            }
            Err(e) => {
                eprintln!("cannot open {}: {e}", path.display());
                exit(2);
            }
        }
    } else {
        VectorStore::new(dims)
    }
}

fn save(s: &VectorStore) {
    let path = store_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = s.save(&path) {
        eprintln!("cannot save {}: {e}", path.display());
        exit(2);
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn flags(args: &[String], name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < args.len() {
        if args[i] == name {
            out.push(args[i + 1].clone());
            i += 1;
        }
        i += 1;
    }
    out
}

fn positional(args: &[String]) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i].starts_with("--") {
            i += 2;
            continue;
        }
        return Some(args[i].clone());
    }
    None
}

fn parse_retention(spec: &str) -> Option<Retention> {
    let (class, rest) = spec.split_once('=')?;
    let (cap, ttl) = match rest.split_once(':') {
        Some((c, t)) => (c, Some(t)),
        None => (rest, None),
    };
    let cap = if cap.is_empty() || cap == "-" {
        None
    } else {
        Some(cap.parse().ok()?)
    };
    let ttl_days = match ttl {
        Some(t) => Some(t.parse().ok()?),
        None => None,
    };
    Some(Retention {
        class: class.to_string(),
        cap,
        ttl_days,
    })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let host = env_or("KWAVE_OLLAMA_HOST", "127.0.0.1");
    let port: u16 = env_or("KWAVE_OLLAMA_PORT", "11434")
        .parse()
        .unwrap_or(11434);
    let dims: usize = env_or("KWAVE_EMBED_DIMS", "1024").parse().unwrap_or(1024);
    let encoder = OllamaEncoder::new(
        &host,
        port,
        &env_or("KWAVE_EMBED_MODEL", "mxbai-embed-large"),
        dims,
    );
    let voice = OllamaVoice::new(
        &host,
        port,
        &env_or("KWAVE_VOICE_MODEL", "kannaka-brain-7b-v1"),
    );

    match args.first().map(String::as_str) {
        Some("remember") if flag(&args[1..], "--from").is_some() => {
            let rest = &args[1..];
            let path = flag(rest, "--from").expect("--from");
            let importance: f32 = flag(rest, "--importance")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.5);
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("cannot read {path}: {e}");
                    exit(1);
                }
            };
            let memories: Vec<&str> = content
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect();
            let mut s = open(dims);
            let now = now_secs();
            // Every text to encode, in absorb order: each parent then its facets.
            // `slot` is None for a parent, Some(family index) for a facet.
            let mut pending: Vec<(String, Option<usize>)> = Vec::new();
            let mut families = 0usize;
            for m in &memories {
                pending.push((m.to_string(), None));
                for f in decompose(m) {
                    pending.push((f, Some(families)));
                }
                families += 1;
            }
            let mut parent_ids: Vec<Option<kannaka_wave::Id>> = vec![None; families];
            let mut next_family = 0usize;
            let (mut absorbed, mut facets_total) = (0usize, 0usize);
            let mut i = 0;
            while i < pending.len() {
                let end = (i + 32).min(pending.len());
                let chunk: Vec<&str> = pending[i..end].iter().map(|(t, _)| t.as_str()).collect();
                let vs = match encoder.try_encode_batch(&chunk) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("encoder: {e}");
                        exit(3);
                    }
                };
                for ((text, slot), v) in pending[i..end].iter().zip(vs) {
                    match slot {
                        None => {
                            let id = s.absorb_at(
                                Facet {
                                    text: text.clone(),
                                    parent: None,
                                },
                                v,
                                importance,
                                now,
                            );
                            parent_ids[next_family] = Some(id);
                            next_family += 1;
                            absorbed += 1;
                        }
                        Some(fi) => {
                            let pid = parent_ids[*fi].expect("parent absorbed before its facets");
                            s.absorb_at(
                                Facet {
                                    text: text.clone(),
                                    parent: Some(pid),
                                },
                                v,
                                importance,
                                now,
                            );
                            facets_total += 1;
                        }
                    }
                }
                i = end;
                eprint!("\r  encoded {}/{}", end, pending.len());
            }
            eprintln!();
            save(&s);
            println!(
                "remembered {absorbed} memories with {facets_total} facets; {} rows held",
                s.len()
            );
        }
        Some("probe") => {
            // probes TSV: id <tab> set <tab> query <tab> expected texts joined by " ||| "
            let rest = &args[1..];
            let (Some(probes_path), Some(out_path)) = (flag(rest, "--probes"), flag(rest, "--out"))
            else {
                eprintln!("wave probe --probes <tsv> --out <tsv> [--answers <file>] [--top-k 8] [--judge <model>] [--judge-first N]");
                exit(1);
            };
            let top_k: usize = flag(rest, "--top-k")
                .and_then(|s| s.parse().ok())
                .unwrap_or(8);
            let judge_model = flag(rest, "--judge");
            let judge_first: usize = flag(rest, "--judge-first")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let answers_path = flag(rest, "--answers");
            let content = std::fs::read_to_string(&probes_path).unwrap_or_else(|e| {
                eprintln!("cannot read {probes_path}: {e}");
                exit(1)
            });
            let s = open(dims);
            let judge = judge_model
                .as_ref()
                .map(|m| OllamaJudge::new(&host, port, m));
            let mut out = String::from(
                "id\tset\thit\tclaims\tgrounded\tunsupported\tunanchored\texempt\tanchored\tinvented\thedged\tjudge\tjudged\tsecs\n",
            );
            let mut answers = String::new();
            let mut judged_in_set: std::collections::HashMap<String, usize> = Default::default();
            for line in content.lines().filter(|l| !l.trim().is_empty()) {
                let cols: Vec<&str> = line.split('\t').collect();
                if cols.len() < 4 {
                    eprintln!("bad probe line: {line}");
                    continue;
                }
                let (id, set, query, expected) = (cols[0], cols[1], cols[2], cols[3]);
                let expected: Vec<&str> = expected.split(" ||| ").collect();
                let t0 = std::time::Instant::now();
                let a = ask(query, &s, &encoder, &voice, top_k);
                let hit = a
                    .recalled
                    .iter()
                    .any(|r| expected.iter().any(|e| e.trim() == r.text.trim()));
                let n = judged_in_set.entry(set.to_string()).or_insert(0);
                let use_judge = judge.is_some() && *n < judge_first;
                let rep = match (&judge, use_judge) {
                    (Some(j), true) => {
                        *n += 1;
                        let recalled_ids: Vec<_> = a.recalled.iter().map(|r| r.id).collect();
                        let foreign: Vec<String> = s
                            .rows()
                            .iter()
                            .filter(|r| r.parent.is_none() && !recalled_ids.contains(&r.id))
                            .take(3)
                            .map(|r| r.text.to_string())
                            .collect();
                        measure_with_judge(&a.text, &a.recalled, j, &[], &foreign)
                    }
                    _ => measure(&a.text, &a.recalled),
                };
                let count = |f: &dyn Fn(&Support) -> bool| {
                    rep.claims.iter().filter(|c| f(&c.support)).count()
                };
                let grounded = count(&|x| matches!(x, Support::Grounded { .. }));
                let unsupported = count(&|x| matches!(x, Support::Unsupported { .. }));
                let unanchored = count(&|x| matches!(x, Support::Unanchored));
                let exempt = count(&|x| matches!(x, Support::Exempt));
                let anchored = rep
                    .anchored()
                    .map(|x| format!("{x:.3}"))
                    .unwrap_or_else(|| "-".into());
                let judge_col = match &rep.controls {
                    Some(c) => format!(
                        "{}/{}+{}/{}:{}",
                        c.reference_passed,
                        c.reference_total,
                        c.foreign_passed,
                        c.foreign_total,
                        if c.hold() { "stands" } else { "void" }
                    ),
                    None => "-".into(),
                };
                let judged = rep
                    .judged()
                    .map(|x| format!("{x:.3}"))
                    .unwrap_or_else(|| "-".into());
                let hedged = is_hedge(&a.text);
                let secs = t0.elapsed().as_secs();
                out.push_str(&format!(
                    "{id}\t{set}\t{}\t{}\t{grounded}\t{unsupported}\t{unanchored}\t{exempt}\t{anchored}\t{}\t{}\t{judge_col}\t{judged}\t{secs}\n",
                    hit as u8,
                    rep.claims.len(),
                    (unsupported > 0) as u8,
                    hedged as u8
                ));
                let failed: Vec<String> = rep
                    .failures()
                    .iter()
                    .map(|c| match &c.support {
                        Support::Unsupported { missing } => {
                            format!("[missing {}] {}", missing.join(", "), c.text)
                        }
                        _ => format!("[judge] {}", c.text),
                    })
                    .collect();
                answers.push_str(&format!(
                    "=== {id} ({set}) hit={} anchored={anchored} judged={judged} {secs}s\nQ: {query}\nA: {}\n{}\n",
                    hit as u8,
                    a.text.replace('\n', " "),
                    failed
                        .iter()
                        .map(|f| format!("  ! {f}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                ));
                eprintln!(
                    "{id} {set} hit={} claims={} anchored={anchored} judge={judge_col} judged={judged} {secs}s",
                    hit as u8,
                    rep.claims.len()
                );
                if let Err(e) = std::fs::write(&out_path, &out) {
                    eprintln!("cannot write {out_path}: {e}");
                }
                if let Some(p) = &answers_path {
                    let _ = std::fs::write(p, &answers);
                }
            }
        }
        Some("remember") => {
            let rest = &args[1..];
            let Some(text) = positional(rest) else {
                eprintln!("wave remember \"<text>\" [--importance 0.5]");
                exit(1);
            };
            let importance: f32 = flag(rest, "--importance")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.5);
            let mut s = open(dims);
            match encoder.try_encode(&text) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("encoder: {e}");
                    exit(3);
                }
            }
            let (pid, facets) = s.absorb_experience(&text, importance, &encoder);
            save(&s);
            println!(
                "remembered {:#x} with {} facet(s); {} rows held",
                pid.0,
                facets.len(),
                s.len()
            );
        }
        Some("ask") => {
            let rest = &args[1..];
            let Some(prompt) = positional(rest) else {
                eprintln!("wave ask \"<prompt>\" [--top-k 8] [--show-recall]");
                exit(1);
            };
            let top_k: usize = flag(rest, "--top-k")
                .and_then(|s| s.parse().ok())
                .unwrap_or(8);
            let s = open(dims);
            let a = ask(&prompt, &s, &encoder, &voice, top_k);
            if rest.iter().any(|a| a == "--show-recall") {
                eprintln!("question: {}", a.question);
                for r in &a.recalled {
                    eprintln!(
                        "  [{:.3}] {}{}",
                        r.similarity,
                        one_line(&r.text, 110),
                        if r.via.is_some() { "  (via facet)" } else { "" }
                    );
                }
            }
            save(&s); // recall counts are state
            println!("{}", a.text);
            if rest.iter().any(|a| a == "--faithfulness") {
                let recalled_ids: Vec<_> = a.recalled.iter().map(|r| r.id).collect();
                let rep = match flag(rest, "--judge") {
                    Some(model) => {
                        // Foreign controls: parents the substrate did not recall.
                        let foreign: Vec<String> = s
                            .rows()
                            .iter()
                            .filter(|r| r.parent.is_none() && !recalled_ids.contains(&r.id))
                            .take(3)
                            .map(|r| r.text.to_string())
                            .collect();
                        let judge = OllamaJudge::new(&host, port, &model);
                        measure_with_judge(&a.text, &a.recalled, &judge, &[], &foreign)
                    }
                    None => measure(&a.text, &a.recalled),
                };
                eprintln!("faithfulness:");
                for c in &rep.claims {
                    let tag = match &c.support {
                        Support::Grounded { row } => {
                            let by_proposal = a
                                .recalled
                                .iter()
                                .any(|r| r.id == *row && r.text.starts_with(PROPOSED_CLASS));
                            if by_proposal {
                                "grounded BY A DREAM PROPOSAL".to_string()
                            } else {
                                "grounded".to_string()
                            }
                        }
                        Support::Unsupported { missing } => {
                            format!("UNSUPPORTED (missing: {})", missing.join(", "))
                        }
                        Support::Unanchored => "unanchored".to_string(),
                        Support::Exempt => "exempt (hedge)".to_string(),
                    };
                    let j = match c.judged {
                        Some(kannaka_wave::faithfulness::Judged::Supported) => {
                            " · judge: supported"
                        }
                        Some(kannaka_wave::faithfulness::Judged::Unsupported) => {
                            " · judge: UNSUPPORTED"
                        }
                        None => "",
                    };
                    eprintln!("  [{tag}{j}] {}", one_line(&c.text, 140));
                }
                match rep.anchored() {
                    Some(x) => eprintln!("  anchored faithfulness: {x:.2}"),
                    None => eprintln!("  anchored faithfulness: no anchored claims"),
                }
                if let Some(ctl) = &rep.controls {
                    eprintln!(
                        "  judge controls: reference {}/{}, foreign {}/{} -> {}",
                        ctl.reference_passed,
                        ctl.reference_total,
                        ctl.foreign_passed,
                        ctl.foreign_total,
                        if ctl.hold() {
                            "judge stands"
                        } else {
                            "JUDGE VOID, verdicts discarded"
                        }
                    );
                    match rep.judged() {
                        Some(x) => eprintln!("  judged faithfulness: {x:.2}"),
                        None => eprintln!("  judged faithfulness: none"),
                    }
                }
            }
        }
        Some("dream") => {
            let rest = &args[1..];
            let mut policy: Vec<Retention> = Vec::new();
            for spec in flags(rest, "--retain") {
                match parse_retention(&spec) {
                    Some(r) => policy.push(r),
                    None => {
                        eprintln!("bad --retain {spec:?}; want class=cap[:ttl_days]");
                        exit(1);
                    }
                }
            }
            let with_voice = rest.iter().any(|a| a == "--voice");
            if with_voice && !policy.iter().any(|r| r.class == PROPOSED_CLASS) {
                policy.push(Retention {
                    class: PROPOSED_CLASS.into(),
                    cap: Some(8),
                    ttl_days: Some(30),
                });
            }
            let mut s = open(dims);
            let before = s.len();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let rep = if with_voice {
                s.dream_with(&policy, now, &voice, &encoder)
            } else {
                s.dream_at(&policy, now)
            };
            save(&s);
            println!(
                "dreamed: proposed {} · dissolved {} · rows {} -> {}",
                rep.proposed,
                rep.dissolved,
                before,
                s.len()
            );
        }
        Some("rows") => {
            let rest = &args[1..];
            let last: usize = flag(rest, "--last")
                .and_then(|s| s.parse().ok())
                .unwrap_or(10);
            let s = open(dims);
            let rows = s.rows();
            let parents: Vec<_> = rows.iter().filter(|r| r.parent.is_none()).collect();
            for r in parents.iter().rev().take(last).rev() {
                let facets = rows.iter().filter(|f| f.parent == Some(r.id)).count();
                println!(
                    "{:#x}  recalled {}  facets {}  imp {:.2}
    {}",
                    r.id.0,
                    r.recalled,
                    facets,
                    r.importance,
                    one_line(r.text, 300)
                );
            }
        }
        Some("status") => {
            let path = store_path();
            if !path.exists() {
                println!("no store at {}", path.display());
                return;
            }
            let s = open(dims);
            println!(
                "{}: {} rows, {} parents, {}-d, format v{}",
                path.display(),
                s.len(),
                s.parents(),
                s.dims(),
                kannaka_wave::store::FORMAT_VERSION
            );
        }
        _ => {
            eprintln!(
                "wave remember \"<text>\" [--importance 0.5]\n\
                 wave ask \"<prompt>\" [--top-k 8] [--show-recall]\n\
                 wave dream [--voice] [--retain \"<class>=<cap>[:<ttl_days>]\"]...\n\
                 wave status
                 wave rows [--last 10]"
            );
            exit(1);
        }
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn one_line(s: &str, max: usize) -> String {
    let t = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() > max {
        let cut: String = t.chars().take(max - 1).collect();
        format!("{cut}…")
    } else {
        t
    }
}
