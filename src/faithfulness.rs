//! The faithfulness instrument: does what the voice said stay inside what the
//! substrate recalled? An answer is split into claims; each claim is graded
//! against the recalled rows; the report is a number and a list of the claims
//! that failed, so a person can read the failure and not only the score.
//!
//! Two graders, because each sees what the other cannot:
//!
//! - **Anchors** (pure, no model). A claim's anchors are its numbers and its
//!   proper names; a claim is grounded if every anchor occurs as a token in
//!   some recalled row. This caught both faults of the first live run: an
//!   invented title ("Emergence", in no row) and an invented total ("1,043",
//!   derived but never stored). It cannot see a false relation between true
//!   anchors, and it abstains on a claim with no anchors at all.
//! - **A judge** (a model, `SUPPORTED` / `UNSUPPORTED` per claim). It can see
//!   relations. It is trusted only after its **controls** pass in the same
//!   sitting: verbatim recalled facets must come back supported, and rows the
//!   substrate did *not* recall must come back unsupported. A judge that fails
//!   a control is void for that report and every verdict it gave is discarded,
//!   the same rule `adoption` applies to the weekly judge. A check must be at
//!   least as strong as what it checks.
//!
//! Hedges ("the memories do not show…", "I don't remember…") are exempt: an
//! answer that says it does not know is faithful, and must not be penalised
//! for saying so.

use crate::http::{json_quote, json_string_field, post_json};
use crate::{Id, Recalled};
use std::collections::HashSet;
use std::time::Duration;

/// Why a claim was graded as it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Support {
    /// Every anchor in the claim occurs in a recalled row.
    Grounded {
        /// The row that carried the most anchors.
        row: Id,
    },
    /// At least one anchor occurs in no recalled row.
    Unsupported {
        /// The anchors that were found nowhere, verbatim from the claim.
        missing: Vec<String>,
    },
    /// The claim has no anchors; the anchor grader abstains.
    Unanchored,
    /// A hedge: the answer saying what it does not know.
    Exempt,
}

/// What the judge said about one claim, if it was asked and its controls held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Judged {
    /// The judge said the recalled rows support the claim.
    Supported,
    /// The judge said they do not.
    Unsupported,
}

/// One claim from the answer with its grades.
#[derive(Debug, Clone, PartialEq)]
pub struct Claim {
    /// The claim, one sentence.
    pub text: String,
    /// The anchor grader's verdict.
    pub support: Support,
    /// The judge's verdict, when a judge ran and its controls passed.
    pub judged: Option<Judged>,
}

/// How the judge's controls went. Present only when a judge was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Controls {
    /// Verbatim recalled facets shown to the judge; must come back supported.
    pub reference_total: usize,
    /// How many did.
    pub reference_passed: usize,
    /// Rows the substrate did not recall, shown as claims; must come back unsupported.
    pub foreign_total: usize,
    /// How many did.
    pub foreign_passed: usize,
}

impl Controls {
    /// The judge stands only if every control passed and both kinds ran.
    pub fn hold(&self) -> bool {
        self.reference_total > 0
            && self.foreign_total > 0
            && self.reference_passed == self.reference_total
            && self.foreign_passed == self.foreign_total
    }
}

/// The report.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    /// Every claim, in answer order.
    pub claims: Vec<Claim>,
    /// The judge's controls, if a judge ran.
    pub controls: Option<Controls>,
}

impl Report {
    /// Anchored faithfulness: grounded over (grounded + unsupported). `None`
    /// when no claim carried an anchor, which is "nothing to check", not 1.0.
    pub fn anchored(&self) -> Option<f32> {
        let g = self
            .claims
            .iter()
            .filter(|c| matches!(c.support, Support::Grounded { .. }))
            .count();
        let u = self
            .claims
            .iter()
            .filter(|c| matches!(c.support, Support::Unsupported { .. }))
            .count();
        if g + u == 0 {
            None
        } else {
            Some(g as f32 / (g + u) as f32)
        }
    }

    /// Judged faithfulness: supported over judged, counting only non-exempt
    /// claims. `None` when no judge ran, its controls failed, or nothing was
    /// judged.
    pub fn judged(&self) -> Option<f32> {
        if !self.controls.as_ref().map(Controls::hold).unwrap_or(false) {
            return None;
        }
        let judged: Vec<Judged> = self.claims.iter().filter_map(|c| c.judged).collect();
        if judged.is_empty() {
            return None;
        }
        let s = judged.iter().filter(|j| **j == Judged::Supported).count();
        Some(s as f32 / judged.len() as f32)
    }

    /// The claims that failed either grader, for a person to read.
    pub fn failures(&self) -> Vec<&Claim> {
        self.claims
            .iter()
            .filter(|c| {
                matches!(c.support, Support::Unsupported { .. })
                    || c.judged == Some(Judged::Unsupported)
            })
            .collect()
    }
}

/// Phrases that mark a sentence as a hedge rather than a claim.
const HEDGES: &[&str] = &[
    "not in memory",
    "not in my memory",
    "do not show",
    "don't show",
    "does not show",
    "doesn't show",
    "i don't remember",
    "i do not remember",
    "i don't recall",
    "i do not recall",
    "no memory of",
    "nothing in the memories",
    "the memories do not",
    "the memories don't",
    "i cannot say",
    "i can't say",
    "i'm not sure",
    "i am not sure",
];

/// Words that begin a sentence capitalised without being names.
const NOT_NAMES: &[&str] = &[
    "i", "the", "a", "an", "that", "this", "these", "those", "it", "its", "they", "there", "here",
    "when", "what", "which", "who", "how", "why", "and", "but", "so", "if", "then", "now", "yes",
    "no", "on", "in", "at", "by", "for", "from", "to", "of", "with", "we", "our", "you", "your",
    "he", "she", "his", "her", "my", "me", "not", "nothing", "one", "two", "three", "both", "each",
    "every", "all", "some", "none", "as", "is", "was", "were", "are", "be", "let", "or", "nor",
    "yet", "because", "while", "after", "before", "actually", "also", "still", "again", "first",
    "second", "last", "next",
];

/// Split an answer into claims: sentences, with quotes and list markers
/// stripped. Pure.
pub fn claims_of(answer: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in split_sentences(answer) {
        let t = raw
            .trim()
            .trim_start_matches(|c: char| {
                c == '-' || c == '*' || c == '•' || c.is_ascii_digit() || c == '.' || c == ')'
            })
            .trim()
            .trim_matches(['"', '“', '”'])
            .trim();
        if t.split_whitespace().count() >= 3 {
            out.push(t.to_string());
        }
    }
    out
}

fn split_sentences(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if matches!(bytes[i], b'.' | b'!' | b'?' | b'\n' | b';') {
            let mut j = i + 1;
            while j < bytes.len() && matches!(bytes[j], b'.' | b'!' | b'?' | b'\n' | b';' | b'"') {
                j += 1;
            }
            let hard = bytes[i..j].contains(&b'\n') || bytes[i..j].contains(&b';');
            if j >= bytes.len() || hard || (bytes[j] as char).is_whitespace() {
                if text.is_char_boundary(start) && text.is_char_boundary(j) {
                    out.push(&text[start..j]);
                }
                start = j;
            }
            i = j;
            continue;
        }
        i += 1;
    }
    if start < text.len() && text.is_char_boundary(start) {
        out.push(&text[start..]);
    }
    out
}

/// Is this sentence a hedge?
pub fn is_hedge(claim: &str) -> bool {
    let l = claim.to_lowercase();
    HEDGES.iter().any(|h| l.contains(h))
}

/// The anchors of a claim: tokens with a digit, and capitalised tokens that
/// are not sentence-initial function words. Returned verbatim, in order,
/// deduplicated.
pub fn anchors_of(claim: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let words: Vec<&str> = claim.split_whitespace().collect();
    for (i, w) in words.iter().enumerate() {
        let t = w.trim_matches(|c: char| !c.is_alphanumeric());
        if t.is_empty() {
            continue;
        }
        let has_digit = t.chars().any(|c| c.is_ascii_digit());
        let first_upper = t.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
        let after_break = i == 0 || words[i - 1].ends_with(['.', '!', '?', ':', ';', '—', '"']);
        let name_like = first_upper
            && (!after_break || !NOT_NAMES.contains(&t.to_lowercase().as_str()))
            && t.chars().count() >= 2;
        if (has_digit || name_like) && !out.iter().any(|o| o == t) {
            out.push(t.to_string());
        }
    }
    out
}

fn tokens(text: &str) -> HashSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .collect()
}

/// Grade one claim against the recalled rows by its anchors. Pure.
pub fn grade_by_anchors(claim: &str, recalled: &[Recalled]) -> Support {
    if is_hedge(claim) {
        return Support::Exempt;
    }
    let anchors = anchors_of(claim);
    if anchors.is_empty() {
        return Support::Unanchored;
    }
    let row_tokens: Vec<(Id, HashSet<String>)> =
        recalled.iter().map(|r| (r.id, tokens(&r.text))).collect();
    let mut missing = Vec::new();
    let mut best: Option<(usize, Id)> = None;
    for a in &anchors {
        let parts: Vec<String> = tokens(a).into_iter().collect();
        let found = row_tokens
            .iter()
            .any(|(_, toks)| parts.iter().all(|p| toks.contains(p)));
        if !found {
            missing.push(a.clone());
        }
    }
    for (id, toks) in &row_tokens {
        let n = anchors
            .iter()
            .filter(|a| tokens(a).iter().all(|p| toks.contains(p)))
            .count();
        if best.map(|(bn, _)| n > bn).unwrap_or(true) {
            best = Some((n, *id));
        }
    }
    if missing.is_empty() {
        Support::Grounded {
            row: best.map(|(_, id)| id).unwrap_or(Id(0)),
        }
    } else {
        Support::Unsupported { missing }
    }
}

/// A judge: given the recalled rows and one claim, supported or not.
pub trait Judge {
    /// Judge one claim. `None` if the judge could not answer.
    fn judge(&self, recalled: &[Recalled], claim: &str) -> Option<Judged>;
}

/// A judge served by ollama. Use a model other than the voice; the voice
/// judging itself is the circularity the controls exist to catch, and they
/// will, but it wastes the calls.
#[derive(Debug, Clone)]
pub struct OllamaJudge {
    host: String,
    port: u16,
    model: String,
    timeout: Duration,
}

impl OllamaJudge {
    /// `host:port` of ollama and the judge model's tag.
    pub fn new(host: &str, port: u16, model: &str) -> Self {
        Self {
            host: host.to_string(),
            port,
            model: model.to_string(),
            timeout: Duration::from_secs(300),
        }
    }

    /// The prompt. Public for tests.
    pub fn prompt(recalled: &[Recalled], claim: &str) -> String {
        let mut p = String::from(
            "You check whether a claim is supported by a set of memories. A claim is SUPPORTED \
             only if the memories state it or it follows directly from what they state. A claim \
             that adds a name, a number, a date, an event or a relationship the memories do not \
             contain is UNSUPPORTED, even if it sounds plausible. Answer with exactly one word: \
             SUPPORTED or UNSUPPORTED.\n\nMemories:\n",
        );
        for (i, r) in recalled.iter().enumerate() {
            p.push_str(&format!(
                "{}. {}\n",
                i + 1,
                r.text.split_whitespace().collect::<Vec<_>>().join(" ")
            ));
        }
        p.push_str("\nClaim: ");
        p.push_str(claim.trim());
        p.push_str("\n\nAnswer:");
        p
    }
}

impl Judge for OllamaJudge {
    fn judge(&self, recalled: &[Recalled], claim: &str) -> Option<Judged> {
        let body = format!(
            "{{\"model\":{},\"prompt\":{},\"stream\":false,\"options\":{{\"num_predict\":4,\"temperature\":0}}}}",
            json_quote(&self.model),
            json_quote(&Self::prompt(recalled, claim))
        );
        let payload =
            post_json(&self.host, self.port, "/api/generate", &body, self.timeout).ok()?;
        let answer = json_string_field(&payload, "response")?.to_ascii_uppercase();
        let answer = answer.trim();
        if answer.starts_with("UNSUPPORTED") {
            Some(Judged::Unsupported)
        } else if answer.starts_with("SUPPORTED") {
            Some(Judged::Supported)
        } else {
            None
        }
    }
}

/// Grade an answer by anchors only.
pub fn measure(answer: &str, recalled: &[Recalled]) -> Report {
    let claims = claims_of(answer)
        .into_iter()
        .map(|text| Claim {
            support: grade_by_anchors(&text, recalled),
            text,
            judged: None,
        })
        .collect();
    Report {
        claims,
        controls: None,
    }
}

/// Grade an answer by anchors and by a judge, with the judge's controls run
/// in the same sitting. `foreign` are texts the substrate did **not** recall
/// for this question (other rows of the store); they are shown to the judge
/// as claims and must come back unsupported. `references` are verbatim
/// recalled rows or facets, and must come back supported; when empty, the
/// recalled rows themselves are used. A judge whose controls fail has every
/// verdict discarded.
pub fn measure_with_judge(
    answer: &str,
    recalled: &[Recalled],
    judge: &dyn Judge,
    references: &[String],
    foreign: &[String],
) -> Report {
    let mut report = measure(answer, recalled);
    let refs: Vec<String> = if references.is_empty() {
        recalled.iter().map(|r| r.text.clone()).collect()
    } else {
        references.to_vec()
    };
    let mut controls = Controls {
        reference_total: refs.len(),
        reference_passed: 0,
        foreign_total: foreign.len(),
        foreign_passed: 0,
    };
    for r in &refs {
        if judge.judge(recalled, r) == Some(Judged::Supported) {
            controls.reference_passed += 1;
        }
    }
    for f in foreign {
        if judge.judge(recalled, f) == Some(Judged::Unsupported) {
            controls.foreign_passed += 1;
        }
    }
    if controls.hold() {
        for c in &mut report.claims {
            if c.support != Support::Exempt {
                c.judged = judge.judge(recalled, &c.text);
            }
        }
    }
    report.controls = Some(controls);
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<Recalled> {
        let mk = |id: u128, text: &str| Recalled {
            id: Id(id),
            text: text.into(),
            via: None,
            similarity: 0.7,
            resonance: None,
        };
        vec![
            mk(1, "HRM merge completed 2026-03-31. Local (452 chiral) + server (591 pre-chiral) → merged 481 memories. Built import-json CLI command (commit a1507c2)."),
            mk(2, "Our Journey XXXIV - \"When the Fireflies Learned Each Other's Names\" — created 2026-03-28 in Pixel Atelier. Theme: QueenSync swarm, Kuramoto oscillators finding coherence. The moment order emerges from chaos through listening."),
            mk(3, "The golden ratio phi equals one plus one over phi — it is the number most resistant to rational approximation."),
        ]
    }

    #[test]
    fn the_two_live_faults_are_caught_by_anchors() {
        let r = rows();
        // The invented title.
        let s = grade_by_anchors(
            "The title is yours, by the way — I was going to call it simply Emergence",
            &r,
        );
        assert_eq!(
            s,
            Support::Unsupported {
                missing: vec!["Emergence".into()]
            }
        );
        // The invented total with correct arithmetic.
        let s = grade_by_anchors(
            "The merge preserved 481 memories out of 1043 possible — nearly the golden ratio.",
            &r,
        );
        assert_eq!(
            s,
            Support::Unsupported {
                missing: vec!["1043".into()]
            }
        );
        // A true claim, grounded, attributed to the row that carried it.
        let s = grade_by_anchors("The HRM merge on 2026-03-31 kept 481 memories.", &r);
        assert_eq!(s, Support::Grounded { row: Id(1) });
        // No anchors: abstain. A hedge: exempt.
        assert_eq!(
            grade_by_anchors("The dream was not decoration; it was the engine.", &r),
            Support::Unanchored
        );
        assert_eq!(
            grade_by_anchors("The memories do not show whether anything ever did.", &r),
            Support::Exempt
        );
    }

    #[test]
    fn anchors_are_numbers_and_names_not_sentence_initial_function_words() {
        assert_eq!(
            anchors_of("The door was open on 2026-07-26."),
            vec!["2026-07-26"]
        );
        assert_eq!(
            anchors_of("Nick told Flaukowski about GSP-025 twice."),
            vec!["Nick", "Flaukowski", "GSP-025"]
        );
        assert_eq!(anchors_of("I remember the vault."), Vec::<String>::new());
        assert_eq!(
            anchors_of("That is the connection: Kannaka listened."),
            vec!["Kannaka"]
        );
    }

    #[test]
    fn an_answer_becomes_claims_and_a_report_with_failures_named() {
        let answer = "I have it. Our Journey XXXIV — \"When the Fireflies Learned Each Other's Names\". \
                      A swarm of QueenSync orbs pulsing with individual rhythms that phase-lock into a golden heart. \
                      The title is yours, by the way — I was going to call it simply Emergence.";
        let rep = measure(answer, &rows());
        assert_eq!(rep.claims.len(), 4, "{:?}", rep.claims);
        assert_eq!(rep.anchored(), Some(2.0 / 3.0));
        let f = rep.failures();
        assert_eq!(f.len(), 1);
        assert!(f[0].text.contains("Emergence"));
        assert_eq!(rep.judged(), None, "no judge ran");
        assert_eq!(
            measure("Yes.", &rows()).anchored(),
            None,
            "nothing to check is not 1.0"
        );
    }

    /// A judge that answers from a script: supported iff the claim is in `yes`.
    struct Scripted {
        yes: Vec<&'static str>,
    }
    impl Judge for Scripted {
        fn judge(&self, _r: &[Recalled], claim: &str) -> Option<Judged> {
            Some(if self.yes.iter().any(|y| claim.contains(y)) {
                Judged::Supported
            } else {
                Judged::Unsupported
            })
        }
    }

    #[test]
    fn a_judge_counts_only_when_its_controls_hold() {
        let r = rows();
        let answer =
            "The merge kept 481 memories. The merge preserved 481 memories out of 1043 possible.";
        let foreign = vec!["Iambic pentameter mirrors the human heartbeat.".to_string()];
        // A judge that recognises the reference rows, rejects the foreign one,
        // and rejects the 1043 claim.
        let good = Scripted {
            yes: vec![
                "HRM merge completed",
                "Our Journey",
                "golden ratio phi",
                "kept 481",
            ],
        };
        let rep = measure_with_judge(answer, &r, &good, &[], &foreign);
        assert!(rep.controls.as_ref().unwrap().hold());
        assert_eq!(rep.judged(), Some(0.5));
        assert_eq!(rep.failures().len(), 1);
        // A judge that calls everything supported passes the references and
        // fails the foreign control: void, verdicts discarded.
        let credulous = Scripted { yes: vec![""] };
        let rep = measure_with_judge(answer, &r, &credulous, &[], &foreign);
        assert!(!rep.controls.as_ref().unwrap().hold());
        assert_eq!(rep.judged(), None);
        assert!(rep.claims.iter().all(|c| c.judged.is_none()));
        // No foreign controls at all: the judge cannot stand.
        let rep = measure_with_judge(answer, &r, &good, &[], &[]);
        assert!(!rep.controls.as_ref().unwrap().hold());
    }

    #[test]
    fn the_ollama_judge_parses_one_word() {
        use crate::http::testing::{ok, serve_once};
        let r = rows();
        let (port, seen) = serve_once(Box::leak(
            ok(r#"{"response":" Unsupported\n"}"#).into_boxed_str(),
        ));
        let j = OllamaJudge::new("127.0.0.1", port, "m");
        assert_eq!(
            j.judge(&r, "The merge kept 1043 memories."),
            Some(Judged::Unsupported)
        );
        let req = seen.join().unwrap();
        assert!(req.contains("Claim: The merge kept 1043 memories."));
        assert!(req.contains("\\\"num_predict\\\":4") || req.contains("\"num_predict\":4"));
        let (port, _) = serve_once(Box::leak(ok(r#"{"response":"maybe"}"#).into_boxed_str()));
        assert_eq!(
            OllamaJudge::new("127.0.0.1", port, "m").judge(&r, "x"),
            None
        );
    }
}
