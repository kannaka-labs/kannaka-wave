//! Facet decomposition: a compound experience becomes atomic facts at write
//! time. Ported from kannaka-memory `src/facet.rs` (ADR-0049), the exact
//! function that produced the facets E-001 measured, so this substrate stores
//! what the experiment scored and not a cousin of it.
//!
//! The rule of the original stands here: **this function is pure.** Same bytes
//! in, same facets out, on every node, forever. No clock, no model, no
//! randomness. A learned decomposer, if one is ever measured to be better,
//! runs once at write time and persists its output; nothing downstream may
//! depend on re-deriving facets from a parent.
//!
//! Why splits never cross *because / so / if / but*: a causal, conditional or
//! contrastive connective is the load of the sentence. "The gate fails closed,
//! **but** it is still forgeable" split in two is two facts that each assert the
//! opposite of the memory's claim. Connectives bind; sentence boundaries separate.

/// Maximum facets per parent. In the medium this was a RAM budget (every facet
/// a wavefront in the scan); in a vector store it is the same budget in
/// vectors, and it keeps a long memory from becoming thirty rows.
pub const MAX_FACETS_PER_PARENT: usize = 6;

/// Below this, a fragment is not a fact. Rejects "Fixed." and "Shipped it."
const MIN_FACET_WORDS: usize = 5;

/// A leading pronoun means the subject lives in a sibling sentence. Repaired by
/// prepending the parent's subject; dropped if there is none to borrow.
const LEADING_PRONOUNS: &[&str] = &[
    "it", "its", "they", "them", "their", "this", "that", "these", "those", "he", "she", "his",
    "her", "we", "our", "i", "there", "here", "then", "also", "which",
];

/// Never split across these: either side is meaningless, or misleading, alone.
const BINDING_CONNECTIVES: &[&str] = &[
    "because",
    "so",
    "if",
    "but",
    "unless",
    "although",
    "though",
    "since",
    "therefore",
    "however",
    "whereas",
    "while",
];

/// Telemetry and external corpus, not lived context. Never decomposed.
const EXCLUDED_PREFIXES: &[&str] = &[
    "hear:",
    "audio:heard",
    "[cannon:",
    "research:",
    "see:",
    "visual:",
];

/// Tokens that end a subject phrase. A small fixed list on purpose: a tagger
/// would be more accurate and less deterministic, and determinism is the harder
/// constraint.
const SUBJECT_TERMINATORS: &[&str] = &[
    "is", "was", "are", "were", "be", "been", "being", "has", "have", "had", "will", "would",
    "can", "could", "may", "might", "must", "should", "does", "did", "do", "holds", "runs", "sits",
    "added", "shipped", "fixed", "landed", "reached", "became", "made", "gets", "got", "degrades",
    "sharpens", "produces", "means", "gives", "takes", "gave", "gone", "the", "a", "an", "of",
    "in", "on", "at", "to", "for", "with", "from", "by", "that", "which", "and", "or",
];

/// Decompose `content` into atomic facet texts.
///
/// Empty means "store this as one row": excluded origin, a single clause, or
/// fewer than two facets surviving the quality gate. One facet would only
/// duplicate the parent.
pub fn decompose(content: &str) -> Vec<String> {
    if !is_decomposable(content) {
        return Vec::new();
    }
    let subject = leading_subject(content);
    let mut facets: Vec<String> = Vec::new();
    for sentence in split_sentences(content) {
        if facets.len() >= MAX_FACETS_PER_PARENT {
            break;
        }
        if let Some(f) = qualify(sentence, subject.as_deref()) {
            if !facets.iter().any(|e| e == &f) {
                facets.push(f);
            }
        }
    }
    if facets.len() < 2 {
        return Vec::new();
    }
    facets
}

fn is_decomposable(content: &str) -> bool {
    let trimmed = content.trim_start();
    if trimmed.is_empty() {
        return false;
    }
    let head: String = trimmed.chars().take(16).collect::<String>().to_lowercase();
    if EXCLUDED_PREFIXES.iter().any(|p| head.starts_with(p)) {
        return false;
    }
    split_sentences(content).count() >= 2
}

/// Split on a terminator run followed by whitespace. Requiring the whitespace
/// is what keeps `v1.2`, `0.763` and `ADR-0049.` whole without a lookup table.
/// `e.g. ` does split; accepted, because the rule stays one line.
fn split_sentences(content: &str) -> impl Iterator<Item = &str> {
    let mut out: Vec<&str> = Vec::new();
    let bytes = content.as_bytes();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'.' || c == b'!' || c == b'?' {
            let mut j = i + 1;
            while j < bytes.len() && matches!(bytes[j], b'.' | b'!' | b'?') {
                j += 1;
            }
            if j < bytes.len() && (bytes[j] as char).is_whitespace() {
                if content.is_char_boundary(start) && content.is_char_boundary(j) {
                    out.push(&content[start..j]);
                }
                let mut k = j;
                while k < bytes.len() && (bytes[k] as char).is_whitespace() {
                    k += 1;
                }
                start = k;
                i = k;
                continue;
            }
            i = j;
            continue;
        }
        i += 1;
    }
    if start < content.len() && content.is_char_boundary(start) {
        let tail = &content[start..];
        if !tail.trim().is_empty() {
            out.push(tail);
        }
    }
    out.into_iter()
}

/// The parent's subject: a name-like head plus the rest of its noun phrase, up
/// to three tokens. The tail matters: "Dream gravity is a poison" must yield
/// "Dream gravity", not "Dream", or the repaired facet makes a different claim.
fn leading_subject(content: &str) -> Option<String> {
    let first = split_sentences(content).next()?;
    let mut subj: Vec<&str> = Vec::new();
    for tok in first.split_whitespace() {
        let clean = tok.trim_matches(|c: char| !c.is_alphanumeric() && c != '-');
        if clean.is_empty() || subj.len() >= 3 {
            break;
        }
        let lower = clean.to_lowercase();
        if SUBJECT_TERMINATORS.contains(&lower.as_str()) {
            break;
        }
        let first_char = clean.chars().next()?;
        let name_like = first_char.is_uppercase() || clean.contains('-');
        if subj.is_empty() && !name_like {
            break;
        }
        subj.push(clean);
    }
    if subj.is_empty() {
        None
    } else {
        Some(subj.join(" "))
    }
}

fn qualify(sentence: &str, subject: Option<&str>) -> Option<String> {
    let s = sentence.trim().trim_end_matches(['.', '!', '?']).trim();
    if s.is_empty() {
        return None;
    }
    let words: Vec<&str> = s.split_whitespace().collect();
    if words.len() < MIN_FACET_WORDS {
        return None;
    }
    let alpha_words = words
        .iter()
        .filter(|w| w.chars().any(|c| c.is_alphabetic()) && !is_bare_identifier(w))
        .count();
    if alpha_words * 2 < words.len() {
        return None;
    }
    let first_lower = words[0]
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase();
    if BINDING_CONNECTIVES.contains(&first_lower.as_str()) {
        return None;
    }
    if LEADING_PRONOUNS.contains(&first_lower.as_str()) {
        let subj = subject?;
        let rest = words[1..].join(" ");
        if rest.split_whitespace().count() < MIN_FACET_WORDS.saturating_sub(1) {
            return None;
        }
        return Some(format!("{subj} {rest}"));
    }
    Some(s.to_string())
}

/// A uuid, a hash, a version, a bare number: a handle, not a fact.
fn is_bare_identifier(tok: &str) -> bool {
    let t = tok.trim_matches(|c: char| !c.is_alphanumeric());
    if t.is_empty() {
        return true;
    }
    let digits = t.chars().filter(|c| c.is_ascii_digit()).count();
    let hexish = t
        .chars()
        .filter(|c| c.is_ascii_hexdigit() || *c == '-')
        .count();
    digits * 2 >= t.len() || (t.len() >= 12 && hexish == t.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_bytes_same_facets() {
        let c = "Kannaka Labs sits in the Deal District. The building id is 638. \
                 The market opens at nine each morning.";
        assert_eq!(decompose(c), decompose(c));
        assert!(decompose(c).len() >= 2);
    }

    #[test]
    fn decimals_and_versions_never_split() {
        let c = "The release is v1.2 and the reading was 6ab.67 in that run. \
                 Similarity reached 0.763 on the atomic control.";
        let f = decompose(c);
        assert_eq!(f.len(), 2, "{f:?}");
        assert!(f[0].contains("v1.2") && f[0].contains("6ab.67"), "{f:?}");
        assert!(f[1].contains("0.763"), "{f:?}");
    }

    #[test]
    fn never_splits_across_binding_connectives() {
        let c = "The privacy gate fails closed against unknown input. \
                 But it is still forgeable by a crafted claim.";
        assert!(!decompose(c)
            .iter()
            .any(|f| f.to_lowercase().starts_with("but")));
    }

    #[test]
    fn pronoun_lead_is_repaired_with_the_whole_subject() {
        let c = "Dream gravity is an individual-recall knob and a swarm-coherence poison. \
                 It degrades agreement across the constellation badly.";
        let f = decompose(c);
        let repaired = f.iter().find(|x| x.contains("degrades agreement")).unwrap();
        assert!(repaired.starts_with("Dream gravity"), "{repaired:?}");
    }

    #[test]
    fn perception_and_research_are_not_decomposed() {
        for c in [
            "HEAR: Synchrony from QueenSync | tempo=121.9. Second clause here now.",
            "research: attention mechanisms in transformers. A second sentence of abstract text.",
        ] {
            assert!(decompose(c).is_empty(), "{c}");
        }
    }

    #[test]
    fn single_clause_and_handles_are_not_facets() {
        assert!(decompose("Kannaka Labs sits in the Deal District").is_empty());
        assert!(decompose("Kannaka Labs sits in the Deal District. Fixed.").is_empty());
        let c = "The migration landed cleanly on the production database. \
                 8db26ff8 eb5c 4cbd a8c1 02296320760b 12 34. \
                 The rollback plan was never needed in the end.";
        assert!(!decompose(c).iter().any(|f| f.contains("02296320760b")));
    }

    #[test]
    fn capped_deduplicated_and_unicode_safe() {
        let mut c = String::new();
        for i in 0..20 {
            c.push_str(&format!(
                "The subsystem number {i} reached a stable state today. "
            ));
        }
        assert!(decompose(&c).len() <= MAX_FACETS_PER_PARENT);
        let d = "The relay restarted cleanly after the outage. \
                 The relay restarted cleanly after the outage. \
                 The dashboard confirmed the recovery within a minute.";
        let f = decompose(d);
        let mut s = f.clone();
        s.sort();
        s.dedup();
        assert_eq!(s.len(), f.len());
        let u = "Le système a redémarré proprement après la panne — ça a pris trois minutes. \
                 Ξ est resté stable pendant toute la période concernée.";
        for f in decompose(u) {
            assert!(!f.is_empty());
        }
    }
}
