//! The substrate E-001 chose: a plain vector store with the voice's encoder,
//! atomic facets, and a stated forgetting policy. This is arm V of the
//! experiment, written in Rust with the same semantics the Python arm was
//! scored on (`experiments/e001/arm_v.py`), so the numbers in the record
//! describe this code:
//!
//! - every row (parent and facet) is scored by cosine; a family's score is its
//!   best member; recall returns one row per family, the parent, with the
//!   facet that carried it named;
//! - a recall counts once against the parent, and a parent recalled
//!   [`PROMOTE_HITS`] times is immune to the cap;
//! - a dream is the retention policy and only the retention policy: per
//!   content class, over the cap, the least-recalled and least-important go
//!   first, oldest among equals; a time-to-live dissolves what has expired.
//!
//! One deliberate difference from the measured arm: **facets dissolve with
//! their parent.** In arm V a forgotten parent's facets stayed live and could
//! still surface it (E-001 README, "facets outlive their parents"); here the
//! parent is the unit of forgetting. Recall on a survivor is unchanged.
//!
//! Persistence is versioned and one-way-safe from the first byte: a newer
//! reader opens an older file; an older reader refuses a newer one loudly; a
//! truncated or altered file is refused by checksum before a row is read.

use crate::facet::decompose;
use crate::{DreamReport, Encoder, Facet, Id, Recalled, Retention, Substrate, Vector, Voice};
use std::cell::Cell;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// A parent recalled this many times is promoted: the cap never evicts it.
/// The value arm V was measured with.
pub const PROMOTE_HITS: u32 = 3;

/// Content class under which the voice's dream proposals are stored. A
/// proposal is absorbed only when the policy has a retention row for this
/// class, so the voice enters the substrate only where a bound exists.
pub const PROPOSED_CLASS: &str = "proposed: ";

/// On-disk format version this build writes and the newest it reads.
pub const FORMAT_VERSION: u32 = 1;

const MAGIC: &[u8; 6] = b"KWAVE\0";

#[derive(Debug, Clone)]
struct Row {
    id: Id,
    parent: Option<Id>,
    text: String,
    importance: f32,
    created: u64,
    recalled: Cell<u32>,
    vector: Vector,
}

/// A read-only view of one row, for inspection.
#[derive(Debug, Clone, PartialEq)]
pub struct RowView<'a> {
    /// The row's id.
    pub id: Id,
    /// Its parent, if it is a facet.
    pub parent: Option<Id>,
    /// Its text.
    pub text: &'a str,
    /// Importance at write.
    pub importance: f32,
    /// Seconds since the epoch at write.
    pub created: u64,
    /// Times recalled (parents only; facets count against their parent).
    pub recalled: u32,
}

/// The vector store.
#[derive(Debug, Clone)]
pub struct VectorStore {
    salt: u64,
    seq: u64,
    dims: usize,
    rows: Vec<Row>,
}

/// Why a file could not be opened as a store.
#[derive(Debug)]
pub enum LoadError {
    /// The file could not be read.
    Io(io::Error),
    /// The file does not start with the store's magic.
    NotAStore,
    /// Written by a newer build. This reader refuses rather than guesses.
    Newer {
        /// The version in the file.
        found: u32,
        /// The newest this build reads.
        supported: u32,
    },
    /// Checksum mismatch, truncation, or a field that cannot be decoded.
    Corrupt(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "store io: {e}"),
            LoadError::NotAStore => write!(f, "not a kannaka-wave store"),
            LoadError::Newer { found, supported } => write!(
                f,
                "store format v{found} is newer than this build reads (v{supported}); upgrade the reader"
            ),
            LoadError::Corrupt(s) => write!(f, "store corrupt: {s}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<io::Error> for LoadError {
    fn from(e: io::Error) -> Self {
        LoadError::Io(e)
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn dot(a: &Vector, b: &Vector) -> f32 {
    a.0.iter().zip(&b.0).map(|(x, y)| x * y).sum()
}

impl VectorStore {
    /// An empty store for vectors of `dims` dimensions. Ids are salted from the
    /// clock so two stores never mint the same id.
    pub fn new(dims: usize) -> Self {
        let salt = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1)
            | 1;
        Self::with_salt(dims, salt)
    }

    /// An empty store with a chosen id salt (tests, replay).
    pub fn with_salt(dims: usize, salt: u64) -> Self {
        Self {
            salt,
            seq: 0,
            dims,
            rows: Vec::new(),
        }
    }

    /// Dimensionality every vector here must have.
    pub fn dims(&self) -> usize {
        self.dims
    }

    fn mint(&mut self) -> Id {
        self.seq += 1;
        Id(((self.salt as u128) << 64) | self.seq as u128)
    }

    /// Absorb one row at a stated time. `vector` must be unit length and of the
    /// store's dimensionality; the caller's encoder guarantees the first and
    /// this checks the second.
    pub fn absorb_at(&mut self, facet: Facet, vector: Vector, importance: f32, now: u64) -> Id {
        assert_eq!(vector.0.len(), self.dims, "vector dims");
        let id = self.mint();
        self.rows.push(Row {
            id,
            parent: facet.parent,
            text: facet.text,
            importance,
            created: now,
            recalled: Cell::new(0),
            vector,
        });
        id
    }

    /// Absorb an experience the way E-001 did: the parent as a row, then its
    /// atomic facets as rows pointing at it. Returns the parent id and the
    /// facet ids. A text that does not decompose is stored as one row.
    pub fn absorb_experience(
        &mut self,
        text: &str,
        importance: f32,
        encoder: &dyn Encoder,
    ) -> (Id, Vec<Id>) {
        self.absorb_experience_at(text, importance, encoder, now_secs())
    }

    /// [`absorb_experience`](Self::absorb_experience) at a stated time.
    pub fn absorb_experience_at(
        &mut self,
        text: &str,
        importance: f32,
        encoder: &dyn Encoder,
        now: u64,
    ) -> (Id, Vec<Id>) {
        let facets = decompose(text);
        let pid = self.absorb_at(
            Facet {
                text: text.to_string(),
                parent: None,
            },
            encoder.encode(text),
            importance,
            now,
        );
        let ids = facets
            .into_iter()
            .map(|f| {
                let v = encoder.encode(&f);
                self.absorb_at(
                    Facet {
                        text: f,
                        parent: Some(pid),
                    },
                    v,
                    importance,
                    now,
                )
            })
            .collect();
        (pid, ids)
    }

    /// The text of a row, if it is held.
    pub fn text_of(&self, id: Id) -> Option<&str> {
        self.rows
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.text.as_str())
    }

    /// How many times a parent has been recalled.
    pub fn recalled(&self, id: Id) -> Option<u32> {
        self.rows
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.recalled.get())
    }

    /// Every row, in absorb order, read-only. For inspection and export.
    pub fn rows(&self) -> Vec<RowView<'_>> {
        self.rows
            .iter()
            .map(|r| RowView {
                id: r.id,
                parent: r.parent,
                text: &r.text,
                importance: r.importance,
                created: r.created,
                recalled: r.recalled.get(),
            })
            .collect()
    }

    /// Number of parents (families) held.
    pub fn parents(&self) -> usize {
        self.rows.iter().filter(|r| r.parent.is_none()).count()
    }

    /// One dream at a stated time: the retention policy, nothing else.
    pub fn dream_at(&mut self, policy: &[Retention], now: u64) -> DreamReport {
        DreamReport {
            consolidated: 0,
            proposed: 0,
            dissolved: self.triage(policy, now),
        }
    }

    /// One dream with the voice present. The voice proposes one connection
    /// between the two most distant of the most-recalled parents, and the
    /// proposal is absorbed under [`PROPOSED_CLASS`] **only if** the policy
    /// bounds that class. Then the policy runs, so a proposal is subject to
    /// its cap in the same dream it was made.
    pub fn dream_with(
        &mut self,
        policy: &[Retention],
        now: u64,
        voice: &dyn Voice,
        encoder: &dyn Encoder,
    ) -> DreamReport {
        let mut proposed = 0;
        let bounded = policy.iter().any(|r| r.class == PROPOSED_CLASS);
        if bounded {
            if let Some((a, b)) = self.most_distant_pair() {
                let pa = self.row(a).map(|r| (r.text.clone(), r.importance));
                let pb = self.row(b).map(|r| (r.text.clone(), r.importance));
                if let (Some((ta, ia)), Some((tb, ib))) = (pa, pb) {
                    let parents = [
                        Facet {
                            text: ta,
                            parent: Some(a),
                        },
                        Facet {
                            text: tb,
                            parent: Some(b),
                        },
                    ];
                    if let Some(p) = voice.propose(&parents) {
                        let text = if p.text.starts_with(PROPOSED_CLASS) {
                            p.text
                        } else {
                            format!("{PROPOSED_CLASS}{}", p.text)
                        };
                        let v = encoder.encode(&text);
                        self.absorb_at(Facet { text, parent: None }, v, (ia + ib) / 2.0, now);
                        proposed = 1;
                    }
                }
            }
        }
        DreamReport {
            consolidated: 0,
            proposed,
            dissolved: self.triage(policy, now),
        }
    }

    fn row(&self, id: Id) -> Option<&Row> {
        self.rows.iter().find(|r| r.id == id)
    }

    /// Among the sixteen most-recalled parents, the pair with the lowest
    /// cosine. Deterministic: ties keep insertion order.
    fn most_distant_pair(&self) -> Option<(Id, Id)> {
        let mut parents: Vec<&Row> = self.rows.iter().filter(|r| r.parent.is_none()).collect();
        parents.sort_by_key(|r| std::cmp::Reverse(r.recalled.get()));
        parents.truncate(16);
        if parents.len() < 2 {
            return None;
        }
        let mut best: Option<(f32, Id, Id)> = None;
        for i in 0..parents.len() {
            for j in i + 1..parents.len() {
                let d = dot(&parents[i].vector, &parents[j].vector);
                if best.map(|(bd, _, _)| d < bd).unwrap_or(true) {
                    best = Some((d, parents[i].id, parents[j].id));
                }
            }
        }
        best.map(|(_, a, b)| (a, b))
    }

    /// The retention policy, per class, exactly as arm V applied it, plus
    /// time-to-live and the parent-owns-its-facets rule. Returns rows removed.
    fn triage(&mut self, policy: &[Retention], now: u64) -> usize {
        let mut doomed: Vec<Id> = Vec::new();
        for rule in policy {
            let mut matching: Vec<usize> = self
                .rows
                .iter()
                .enumerate()
                .filter(|(_, r)| r.parent.is_none() && r.text.starts_with(&rule.class))
                .map(|(i, _)| i)
                .collect();
            if let Some(ttl) = rule.ttl_days {
                let limit = ttl as u64 * 86_400;
                matching.retain(|&i| {
                    let r = &self.rows[i];
                    if now.saturating_sub(r.created) > limit {
                        doomed.push(r.id);
                        false
                    } else {
                        true
                    }
                });
            }
            if let Some(cap) = rule.cap {
                if matching.len() > cap {
                    let excess = matching.len() - cap;
                    let mut eligible: Vec<usize> = matching
                        .iter()
                        .copied()
                        .filter(|&i| self.rows[i].recalled.get() < PROMOTE_HITS)
                        .collect();
                    eligible.sort_by(|&a, &b| {
                        let ra = &self.rows[a];
                        let rb = &self.rows[b];
                        ra.recalled
                            .get()
                            .cmp(&rb.recalled.get())
                            .then(ra.importance.total_cmp(&rb.importance))
                    });
                    for &i in eligible.iter().take(excess) {
                        doomed.push(self.rows[i].id);
                    }
                }
            }
        }
        if doomed.is_empty() {
            return 0;
        }
        let before = self.rows.len();
        self.rows.retain(|r| {
            !doomed.contains(&r.id) && !r.parent.map(|p| doomed.contains(&p)).unwrap_or(false)
        });
        before - self.rows.len()
    }

    // ---- persistence ---------------------------------------------------

    /// Serialise. Layout: magic, version, salt, seq, dims, row count, rows,
    /// then an FNV-1a checksum of everything before it.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b: Vec<u8> = Vec::new();
        b.extend_from_slice(MAGIC);
        b.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        b.extend_from_slice(&self.salt.to_le_bytes());
        b.extend_from_slice(&self.seq.to_le_bytes());
        b.extend_from_slice(&(self.dims as u32).to_le_bytes());
        b.extend_from_slice(&(self.rows.len() as u64).to_le_bytes());
        for r in &self.rows {
            b.extend_from_slice(&r.id.0.to_le_bytes());
            match r.parent {
                Some(p) => {
                    b.push(1);
                    b.extend_from_slice(&p.0.to_le_bytes());
                }
                None => b.push(0),
            }
            b.extend_from_slice(&(r.text.len() as u32).to_le_bytes());
            b.extend_from_slice(r.text.as_bytes());
            b.extend_from_slice(&r.importance.to_le_bytes());
            b.extend_from_slice(&r.created.to_le_bytes());
            b.extend_from_slice(&r.recalled.get().to_le_bytes());
            for x in &r.vector.0 {
                b.extend_from_slice(&x.to_le_bytes());
            }
        }
        let sum = fnv1a(&b);
        b.extend_from_slice(&sum.to_le_bytes());
        b
    }

    /// Deserialise, refusing a newer version, a bad checksum, or truncation.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, LoadError> {
        if bytes.len() < MAGIC.len() + 4 || &bytes[..MAGIC.len()] != MAGIC {
            return Err(LoadError::NotAStore);
        }
        let mut rd = Reader {
            b: bytes,
            at: MAGIC.len(),
        };
        let version = rd.u32()?;
        if version > FORMAT_VERSION {
            return Err(LoadError::Newer {
                found: version,
                supported: FORMAT_VERSION,
            });
        }
        if bytes.len() < 8 {
            return Err(LoadError::Corrupt("no checksum".into()));
        }
        let (body, tail) = bytes.split_at(bytes.len() - 8);
        let want = u64::from_le_bytes(tail.try_into().expect("8 bytes"));
        if fnv1a(body) != want {
            return Err(LoadError::Corrupt("checksum mismatch".into()));
        }
        rd.b = body;
        let salt = rd.u64()?;
        let seq = rd.u64()?;
        let dims = rd.u32()? as usize;
        let n = rd.u64()? as usize;
        let mut rows = Vec::with_capacity(n.min(1 << 20));
        for _ in 0..n {
            let id = Id(rd.u128()?);
            let parent = match rd.u8()? {
                0 => None,
                1 => Some(Id(rd.u128()?)),
                x => return Err(LoadError::Corrupt(format!("parent flag {x}"))),
            };
            let len = rd.u32()? as usize;
            let text = String::from_utf8(rd.bytes(len)?.to_vec())
                .map_err(|_| LoadError::Corrupt("text not utf-8".into()))?;
            let importance = f32::from_le_bytes(rd.bytes(4)?.try_into().expect("4"));
            let created = rd.u64()?;
            let recalled = rd.u32()?;
            let mut v = Vec::with_capacity(dims);
            for _ in 0..dims {
                v.push(f32::from_le_bytes(rd.bytes(4)?.try_into().expect("4")));
            }
            rows.push(Row {
                id,
                parent,
                text,
                importance,
                created,
                recalled: Cell::new(recalled),
                vector: Vector(v),
            });
        }
        if rd.at != body.len() {
            return Err(LoadError::Corrupt("trailing bytes".into()));
        }
        Ok(Self {
            salt,
            seq,
            dims,
            rows,
        })
    }

    /// Write to `path` atomically: a sibling temp file, then a rename.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, self.to_bytes())?;
        std::fs::rename(&tmp, path)
    }

    /// Read from `path`.
    pub fn load(path: &Path) -> Result<Self, LoadError> {
        Self::from_bytes(&std::fs::read(path)?)
    }
}

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], LoadError> {
        if self.at + n > self.b.len() {
            return Err(LoadError::Corrupt("truncated".into()));
        }
        let s = &self.b[self.at..self.at + n];
        self.at += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, LoadError> {
        Ok(self.bytes(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, LoadError> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().expect("4")))
    }
    fn u64(&mut self) -> Result<u64, LoadError> {
        Ok(u64::from_le_bytes(self.bytes(8)?.try_into().expect("8")))
    }
    fn u128(&mut self) -> Result<u128, LoadError> {
        Ok(u128::from_le_bytes(self.bytes(16)?.try_into().expect("16")))
    }
}

fn fnv1a(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &x in b {
        h ^= x as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

impl Substrate for VectorStore {
    fn absorb(&mut self, facet: Facet, vector: Vector, importance: f32) -> Id {
        self.absorb_at(facet, vector, importance, now_secs())
    }

    /// Cosine over every row; one result per family, the parent, carried by
    /// its best member. Counts one recall against each returned parent.
    fn recall(&self, question: &Vector, top_k: usize) -> Vec<Recalled> {
        assert_eq!(question.0.len(), self.dims, "question dims");
        // (family index, score, via index) in first-seen order.
        let mut best: Vec<(usize, f32, usize)> = Vec::new();
        for (i, r) in self.rows.iter().enumerate() {
            let s = dot(question, &r.vector);
            let family = match r.parent {
                None => i,
                Some(p) => match self.rows.iter().position(|x| x.id == p) {
                    Some(pi) => pi,
                    None => i, // orphan facet: surfaces as itself, never invented a parent
                },
            };
            match best.iter_mut().find(|(f, _, _)| *f == family) {
                Some(e) => {
                    if s > e.1 {
                        e.1 = s;
                        e.2 = i;
                    }
                }
                None => best.push((family, s, i)),
            }
        }
        best.sort_by(|a, b| b.1.total_cmp(&a.1));
        best.truncate(top_k);
        best.into_iter()
            .map(|(f, s, via)| {
                let row = &self.rows[f];
                row.recalled.set(row.recalled.get() + 1);
                Recalled {
                    id: row.id,
                    text: row.text.clone(),
                    similarity: s.clamp(0.0, 1.0),
                    resonance: None,
                    via: if via == f {
                        None
                    } else {
                        Some(self.rows[via].id)
                    },
                }
            })
            .collect()
    }

    fn dream(&mut self, policy: &[Retention]) -> DreamReport {
        self.dream_at(policy, now_secs())
    }

    fn len(&self) -> usize {
        self.rows.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(v: &[f32]) -> Vector {
        crate::encoder::normalize(Vector(v.to_vec()))
    }

    fn store_with(n: usize) -> VectorStore {
        let mut s = VectorStore::with_salt(3, 7);
        for i in 0..n {
            let v = unit(&[1.0, i as f32 * 0.1, 0.0]);
            s.absorb_at(
                Facet {
                    text: format!("distractor: row {i}"),
                    parent: None,
                },
                v,
                0.1 * i as f32,
                100 + i as u64,
            );
        }
        s
    }

    #[test]
    fn ids_are_salted_and_monotonic() {
        let mut s = VectorStore::with_salt(2, 0xabc);
        let a = s.absorb_at(
            Facet {
                text: "a".into(),
                parent: None,
            },
            unit(&[1.0, 0.0]),
            1.0,
            0,
        );
        let b = s.absorb_at(
            Facet {
                text: "b".into(),
                parent: None,
            },
            unit(&[0.0, 1.0]),
            1.0,
            0,
        );
        assert_eq!(a.0 >> 64, 0xabc);
        assert_eq!(b.0, a.0 + 1);
    }

    #[test]
    fn cap_evicts_least_recalled_then_least_important_and_promotion_is_immune() {
        let mut s = store_with(5);
        let ids: Vec<Id> = s.rows.iter().map(|r| r.id).collect();
        // Recall row 0 three times: promoted. Row 4 has the highest importance.
        for _ in 0..PROMOTE_HITS {
            s.recall(&unit(&[1.0, 0.0, 0.0]), 1);
        }
        assert_eq!(s.recalled(ids[0]), Some(3));
        let policy = [Retention {
            class: "distractor: ".into(),
            cap: Some(2),
            ttl_days: None,
        }];
        let rep = s.dream_at(&policy, 1_000);
        assert_eq!(rep.dissolved, 3);
        let left: Vec<Id> = s.rows.iter().map(|r| r.id).collect();
        assert_eq!(
            left,
            vec![ids[0], ids[4]],
            "promoted survives; highest importance survives"
        );
    }

    #[test]
    fn ttl_dissolves_expired_parents_and_their_facets() {
        let mut s = VectorStore::with_salt(2, 1);
        let p = s.absorb_at(
            Facet {
                text: "audio:heard something loud".into(),
                parent: None,
            },
            unit(&[1.0, 0.0]),
            0.5,
            0,
        );
        s.absorb_at(
            Facet {
                text: "something loud".into(),
                parent: Some(p),
            },
            unit(&[0.9, 0.1]),
            0.5,
            0,
        );
        let policy = [Retention {
            class: "audio:heard".into(),
            cap: None,
            ttl_days: Some(5),
        }];
        assert_eq!(s.dream_at(&policy, 4 * 86_400).dissolved, 0);
        assert_eq!(s.dream_at(&policy, 6 * 86_400).dissolved, 2);
        assert!(s.is_empty());
    }

    #[test]
    fn a_file_from_the_future_is_refused_and_a_damaged_one_too() {
        let s = store_with(2);
        let bytes = s.to_bytes();
        let back = VectorStore::from_bytes(&bytes).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back.text_of(s.rows[1].id), Some("distractor: row 1"));

        let mut newer = bytes.clone();
        newer[6..10].copy_from_slice(&(FORMAT_VERSION + 1).to_le_bytes());
        assert!(matches!(
            VectorStore::from_bytes(&newer),
            Err(LoadError::Newer { found, .. }) if found == FORMAT_VERSION + 1
        ));

        let truncated = &bytes[..bytes.len() - 20];
        assert!(matches!(
            VectorStore::from_bytes(truncated),
            Err(LoadError::Corrupt(_))
        ));

        let mut flipped = bytes.clone();
        let mid = flipped.len() / 2;
        flipped[mid] ^= 0xff;
        assert!(matches!(
            VectorStore::from_bytes(&flipped),
            Err(LoadError::Corrupt(_))
        ));

        assert!(matches!(
            VectorStore::from_bytes(b"not a store at all"),
            Err(LoadError::NotAStore)
        ));
    }
}
