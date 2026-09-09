//! The substrate end to end with a deterministic toy encoder: hashed bag of
//! words in 64 dimensions. Not a semantic encoder; enough to prove the
//! plumbing that E-001 measured is the plumbing that ships.

use kannaka_wave::encoder::normalize;
use kannaka_wave::store::{VectorStore, PROPOSED_CLASS};
use kannaka_wave::{Encoder, Facet, Recalled, Retention, Substrate, Vector, Voice};

struct BagOfWords;

impl Encoder for BagOfWords {
    fn encode(&self, text: &str) -> Vector {
        let mut v = vec![0.0f32; 64];
        for w in text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2)
        {
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for b in w.to_lowercase().bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
            v[(h % 64) as usize] += 1.0;
        }
        normalize(Vector(v))
    }
    fn dims(&self) -> usize {
        64
    }
}

const COMPOUND: &str = "Kannaka Labs sits in the Deal District beside the northern square. \
                        The escrow vault runs on the same block every trading morning. \
                        Vincent published the column about the city on a Tuesday.";

#[test]
fn a_compound_memory_is_recalled_through_its_facet_and_resolved_to_its_parent() {
    let enc = BagOfWords;
    let mut s = VectorStore::new(64);
    let (pid, facets) = s.absorb_experience(COMPOUND, 0.8, &enc);
    assert!(facets.len() >= 2, "no decomposition: {facets:?}");
    assert_eq!(s.len(), 1 + facets.len());

    // Something unrelated, so the ranking has a loser.
    s.absorb_experience(
        "The relay restarted cleanly after the outage last night.",
        0.5,
        &enc,
    );

    let q = enc.encode("escrow vault trading morning block");
    let got = s.recall(&q, 5);
    assert_eq!(got[0].id, pid, "best family is the compound's parent");
    assert_eq!(got[0].text, COMPOUND, "the parent's full text comes back");
    assert!(
        got[0].via.is_some(),
        "carried by a facet, and the facet is named"
    );
    assert!(
        got.iter().filter(|r| r.id == pid).count() == 1,
        "one row per family"
    );
    assert!(
        got[0].resonance.is_none(),
        "a vector store has no resonance and says so"
    );
    assert_eq!(
        s.recalled(pid),
        Some(1),
        "one recall counted against the parent, not per facet"
    );
}

#[test]
fn forgetting_is_exercised_and_survivors_are_the_policy_says() {
    let enc = BagOfWords;
    let mut s = VectorStore::new(64);
    let keep = s
        .absorb_experience(
            "The charter names five refusals and the steward keeps them.",
            1.0,
            &enc,
        )
        .0;
    const NAMES: [&str; 10] = [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
    ];
    let mut ds = Vec::new();
    for name in NAMES {
        ds.push(
            s.absorb_experience(
                &format!("distractor: a passing detail called {name} nobody asked about."),
                0.2,
                &enc,
            )
            .0,
        );
    }
    // Recall distractor three three times: promoted.
    for _ in 0..3 {
        let q = enc.encode("passing detail called three nobody asked about");
        let top = s.recall(&q, 1);
        assert_eq!(top[0].id, ds[3]);
    }
    let policy = [Retention {
        class: "distractor: ".into(),
        cap: Some(4),
        ttl_days: None,
    }];
    let rep = s.dream(&policy);
    assert!(
        rep.dissolved >= 6,
        "cap 4 over 10 must forget six parents: {rep:?}"
    );
    assert!(s.text_of(keep).is_some(), "uncapped class untouched");
    assert!(
        s.text_of(ds[3]).is_some(),
        "the promoted distractor survives"
    );
    assert_eq!(s.parents(), 1 + 4);
}

struct Connector;

impl Voice for Connector {
    fn speak(&self, _q: &str, _f: &[Recalled]) -> String {
        String::new()
    }
    fn propose(&self, parents: &[Facet]) -> Option<Facet> {
        Some(Facet {
            text: format!(
                "{} relates to {}",
                first_words(&parents[0].text),
                first_words(&parents[1].text)
            ),
            parent: None,
        })
    }
}

fn first_words(s: &str) -> String {
    s.split_whitespace().take(3).collect::<Vec<_>>().join(" ")
}

#[test]
fn the_voice_enters_only_where_the_policy_bounds_it() {
    let enc = BagOfWords;
    let mut s = VectorStore::new(64);
    s.absorb_experience(
        "The gate fails closed against unknown input by design.",
        0.7,
        &enc,
    );
    s.absorb_experience(
        "Rain fell on the eastern harbour for most of the afternoon.",
        0.7,
        &enc,
    );

    let unbounded = [Retention {
        class: "distractor: ".into(),
        cap: Some(10),
        ttl_days: None,
    }];
    let rep = s.dream_with(&unbounded, 1_000, &Connector, &enc);
    assert_eq!(
        rep.proposed, 0,
        "no retention row for proposals: the voice stays out"
    );

    let bounded = [Retention {
        class: PROPOSED_CLASS.into(),
        cap: Some(1),
        ttl_days: None,
    }];
    let rep = s.dream_with(&bounded, 1_000, &Connector, &enc);
    assert_eq!(rep.proposed, 1);
    let rep = s.dream_with(&bounded, 2_000, &Connector, &enc);
    assert_eq!(rep.proposed, 1);
    assert_eq!(
        rep.dissolved, 1,
        "the second proposal is capped in the same dream"
    );
    let proposals = (0..s.len()).count();
    assert_eq!(proposals, 3, "two parents and exactly one proposal held");
}

#[test]
fn save_and_load_round_trip_keeps_recall_and_counts() {
    let enc = BagOfWords;
    let dir = std::env::temp_dir().join(format!("kannaka-wave-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("store.kwave");

    let mut s = VectorStore::new(64);
    let (pid, _) = s.absorb_experience(COMPOUND, 0.8, &enc);
    let q = enc.encode("escrow vault trading morning block");
    s.recall(&q, 1);
    s.save(&path).unwrap();

    let back = VectorStore::load(&path).unwrap();
    assert_eq!(back.len(), s.len());
    assert_eq!(back.recalled(pid), Some(1));
    let got = back.recall(&q, 1);
    assert_eq!(got[0].id, pid);
    std::fs::remove_dir_all(&dir).unwrap();
}
