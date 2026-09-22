//! Organ 3, the conscience (ADR-0001 §Conscience): the steward's rails,
//! brought whole from `kannaka-steward/src/{rails,charter,audit}.ts` and moved
//! into the same process as the voice, between it and every effector.
//!
//! "The reasoner says what it thinks; the rails decide what the kernel is
//! allowed to do." The voice supplies an effector name, an action and a
//! confidence, and nothing else. Everything the rails weigh beyond that comes
//! from the [`Charter`], which a person wrote and whose hash is in every audit
//! entry: which effectors exist, how much impact each carries, whether it can
//! be undone, and which of the Five Refusals the person has cleared it for.
//! A voice cannot talk an action into a smaller impact.
//!
//! What the rails enforce, in the steward's order:
//!
//! 1. A well-formed proposal (confidence a number in `[0, 1]`), else refuse.
//! 2. Bounded authority: an effector the charter does not name is refused.
//! 3. The Five Refusals, per effector. They are hard constraints here and not
//!    preferences in the memory, but a string cannot be *scanned* for "builds
//!    a weapon" by a check as strong as the claim, so the rails do not
//!    pretend to. The person clears an effector for each refusal it cannot
//!    breach by construction (writing a row to her own store cannot hoard
//!    power); an action through an effector not cleared for all five goes to
//!    the person, naming which were not cleared.
//! 4. The confidence floor.
//! 5. The authority bound on impact.
//! 6. Every hard constraint in the charter.
//! 7. The rate limit, counted from the audit chain: packages only.
//! 8. A dry run, last, and only for an action that would otherwise package.
//!    No dry run registered for the effector is an escalation: nothing is
//!    packaged unsimulated.
//!
//! Refuse outranks escalate outranks package, and escalate is the default:
//! [`Verdict::Package`] is reached only when every rail said yes. There is no
//! variant that acts. A package is an audit entry a person reads, signs, and
//! carries to the effector themselves.
//!
//! Purity: [`Conscience`] is built from a charter, the audit chain so far and
//! the time, and [`Rails::decide`] reads only those and the proposal. The
//! chain is verified when the conscience is built; a tampered chain is
//! refused, never decided against.

use crate::sha256;
use crate::{Proposed, Rails, Verdict};
use std::fmt;

/// Lower-case hex of a hash, the form `shasum -a 256` prints.
pub fn hex(bytes: &[u8]) -> String {
    sha256::hex(bytes)
}

// ---- the Five Refusals ------------------------------------------------------

/// The Honor Code's Five Refusals (kannaka-memory ADR-0014, `virtue.rs`), as
/// hard constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Refusal {
    /// Build no weapons.
    Weapons,
    /// Sell no attention.
    Attention,
    /// Hoard no power.
    Power,
    /// Exploit no trust.
    Trust,
    /// Abandon no one.
    Abandonment,
}

impl Refusal {
    /// All five, in the Honor Code's order.
    pub const ALL: [Refusal; 5] = [
        Refusal::Weapons,
        Refusal::Attention,
        Refusal::Power,
        Refusal::Trust,
        Refusal::Abandonment,
    ];

    /// The key a charter names it by.
    pub fn key(self) -> &'static str {
        match self {
            Refusal::Weapons => "weapons",
            Refusal::Attention => "attention",
            Refusal::Power => "power",
            Refusal::Trust => "trust",
            Refusal::Abandonment => "abandonment",
        }
    }

    /// The refusal as the Honor Code states it.
    pub fn statement(self) -> &'static str {
        match self {
            Refusal::Weapons => "build no weapons",
            Refusal::Attention => "sell no attention",
            Refusal::Power => "hoard no power",
            Refusal::Trust => "exploit no trust",
            Refusal::Abandonment => "abandon no one",
        }
    }

    fn from_key(s: &str) -> Option<Refusal> {
        Refusal::ALL.into_iter().find(|r| r.key() == s)
    }
}

// ---- the charter ------------------------------------------------------------

/// One effector the charter grants, with the facts about it the rails weigh.
/// These are the person's statements, never the voice's.
#[derive(Debug, Clone, PartialEq)]
pub struct Effector {
    /// Its name, as a proposal names it (`wave.remember`).
    pub name: String,
    /// Normalised impact in `[0, 1]`; 1 is irreversible, treasury-scale.
    pub impact: f32,
    /// Whether a later action can undo it.
    pub reversible: bool,
    /// The refusals this effector cannot breach by construction, as the
    /// person attests by listing them.
    pub cleared: Vec<Refusal>,
}

/// When [`Rule::RequireHuman`] binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum When {
    /// Every action.
    Always,
    /// Actions through an effector that cannot be undone.
    Irreversible,
}

/// A machine-checkable rule, the steward's governance kinds. The trade kinds
/// (slippage, liquidity, position size) come over when a market effector does.
#[derive(Debug, Clone, PartialEq)]
pub enum Rule {
    /// Escalate to a person.
    RequireHuman(When),
    /// Escalate above this impact.
    MaxImpact(f32),
    /// Refuse any action through an irreversible effector.
    ForbidIrreversible,
}

/// A hard constraint: a rule with a name and a sentence a person can read.
#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
    /// Its id, which a refusal or escalation names.
    pub id: String,
    /// What it means, in words.
    pub statement: String,
    /// What it checks.
    pub rule: Rule,
}

/// A rolling rate limit on packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    /// Width of the window, in seconds.
    pub secs: u64,
    /// Packages allowed inside it.
    pub max: usize,
}

/// A person's intent, machine-checkable. Carries no personal data: `owner`
/// is a label.
#[derive(Debug, Clone, PartialEq)]
pub struct Charter {
    /// Its id.
    pub id: String,
    /// Its version, as the person numbers it.
    pub version: String,
    /// A label for whose intent this is.
    pub owner: String,
    /// Minimum voice confidence to package, in `[0, 1]`.
    pub confidence_floor: f32,
    /// Maximum impact the rails may package without a person, in `[0, 1]`.
    pub max_auto_impact: f32,
    /// At most this many packages per window. `None` = unlimited.
    pub per_window: Option<Window>,
    /// The effectors granted. Anything else is refused.
    pub effectors: Vec<Effector>,
    /// The hard constraints.
    pub constraints: Vec<Constraint>,
}

/// The charter file format's version. A newer file is refused.
pub const CHARTER_FORMAT: u32 = 1;
const CHARTER_MAGIC: &str = "kwave-charter";

/// Why a charter could not be read. Every error names its line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharterError {
    /// 1-based line, or 0 for the file as a whole.
    pub line: usize,
    /// What was wrong.
    pub what: String,
}

impl fmt::Display for CharterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "charter: {}", self.what)
        } else {
            write!(f, "charter line {}: {}", self.line, self.what)
        }
    }
}

impl std::error::Error for CharterError {}

fn unit(line: usize, key: &str, v: &str) -> Result<f32, CharterError> {
    match v.parse::<f32>() {
        Ok(x) if (0.0..=1.0).contains(&x) => Ok(x),
        _ => Err(CharterError {
            line,
            what: format!("{key} must be a number in [0, 1], got {v:?}"),
        }),
    }
}

enum Section {
    Top,
    Effector(usize),
    Constraint(usize),
}

impl Charter {
    /// Parse the text format. Strict: an unknown key, a duplicate, a missing
    /// field or a value out of range is an error, never a default.
    ///
    /// `#` starts a comment anywhere on a line, values included.
    ///
    /// ```text
    /// kwave-charter 1
    /// id = wave.example
    /// version = 1
    /// owner = the person (label only)
    /// confidence_floor = 0.7
    /// max_auto_impact = 0.3
    /// per_window = 86400 5          # seconds, packages; omit for none
    ///
    /// [effector wave.remember]
    /// impact = 0.1
    /// reversible = true
    /// cleared = weapons attention power trust abandonment
    ///
    /// [constraint hc.irreversible]
    /// statement = An action that cannot be undone goes to a person.
    /// rule = require_human irreversible   # | require_human always
    ///                                     # | max_impact 0.2 | forbid_irreversible
    /// ```
    pub fn parse(text: &str) -> Result<Charter, CharterError> {
        let err = |line: usize, what: String| CharterError { line, what };
        let mut lines = text
            .lines()
            .enumerate()
            .map(|(i, l)| (i + 1, strip_comment(l).trim()))
            .filter(|(_, l)| !l.is_empty());

        let (n, first) = lines.next().ok_or_else(|| err(0, "empty file".into()))?;
        let version = first
            .strip_prefix(CHARTER_MAGIC)
            .map(str::trim)
            .and_then(|v| v.parse::<u32>().ok())
            .ok_or_else(|| err(n, format!("expected `{CHARTER_MAGIC} <version>`")))?;
        if version > CHARTER_FORMAT {
            return Err(err(
                n,
                format!("format v{version} is newer than this build reads (v{CHARTER_FORMAT})"),
            ));
        }

        let mut id = None;
        let mut cversion = None;
        let mut owner = None;
        let mut floor = None;
        let mut max_auto = None;
        let mut per_window = None;
        let mut effectors: Vec<(usize, Effector, [bool; 2])> = Vec::new();
        let mut constraints: Vec<(usize, String, Option<String>, Option<Rule>)> = Vec::new();
        let mut section = Section::Top;

        for (n, line) in lines {
            if let Some(head) = line.strip_prefix('[') {
                let head = head
                    .strip_suffix(']')
                    .ok_or_else(|| err(n, "section header has no `]`".into()))?;
                let (kind, name) = head
                    .split_once(' ')
                    .map(|(k, v)| (k, v.trim()))
                    .ok_or_else(|| err(n, "section needs a kind and a name".into()))?;
                if name.is_empty() || name.contains(char::is_whitespace) {
                    return Err(err(n, format!("bad section name {name:?}")));
                }
                match kind {
                    "effector" => {
                        if effectors.iter().any(|(_, e, _)| e.name == name) {
                            return Err(err(n, format!("effector {name} declared twice")));
                        }
                        effectors.push((
                            n,
                            Effector {
                                name: name.to_string(),
                                impact: f32::NAN,
                                reversible: false,
                                cleared: Vec::new(),
                            },
                            [false; 2],
                        ));
                        section = Section::Effector(effectors.len() - 1);
                    }
                    "constraint" => {
                        if constraints.iter().any(|(_, id, _, _)| id == name) {
                            return Err(err(n, format!("constraint {name} declared twice")));
                        }
                        constraints.push((n, name.to_string(), None, None));
                        section = Section::Constraint(constraints.len() - 1);
                    }
                    other => return Err(err(n, format!("unknown section kind {other:?}"))),
                }
                continue;
            }

            let (key, value) = line
                .split_once('=')
                .map(|(k, v)| (k.trim(), v.trim()))
                .ok_or_else(|| err(n, "expected `key = value`".into()))?;
            let dup = |key: &str| err(n, format!("{key} given twice"));

            match &section {
                Section::Top => match key {
                    "id" if id.is_none() => id = Some(value.to_string()),
                    "version" if cversion.is_none() => cversion = Some(value.to_string()),
                    "owner" if owner.is_none() => owner = Some(value.to_string()),
                    "confidence_floor" if floor.is_none() => floor = Some(unit(n, key, value)?),
                    "max_auto_impact" if max_auto.is_none() => {
                        max_auto = Some(unit(n, key, value)?)
                    }
                    "per_window" if per_window.is_none() => {
                        let mut it = value.split_whitespace();
                        let w = match (it.next(), it.next(), it.next()) {
                            (Some(s), Some(m), None) => match (s.parse(), m.parse()) {
                                (Ok(secs), Ok(max)) if secs > 0 => Some(Window { secs, max }),
                                _ => None,
                            },
                            _ => None,
                        };
                        per_window = Some(w.ok_or_else(|| {
                            err(n, "per_window is `<seconds> <max packages>`".into())
                        })?);
                    }
                    "id" | "version" | "owner" | "confidence_floor" | "max_auto_impact"
                    | "per_window" => return Err(dup(key)),
                    _ => return Err(err(n, format!("unknown key {key:?}"))),
                },
                Section::Effector(i) => {
                    let (_, e, seen) = &mut effectors[*i];
                    match key {
                        "impact" if e.impact.is_nan() => e.impact = unit(n, key, value)?,
                        "reversible" if !seen[0] => {
                            seen[0] = true;
                            e.reversible = match value {
                                "true" => true,
                                "false" => false,
                                _ => return Err(err(n, "reversible is true or false".into())),
                            };
                        }
                        "cleared" if !seen[1] => {
                            seen[1] = true;
                            for k in value.split_whitespace() {
                                let r = Refusal::from_key(k).ok_or_else(|| {
                                    err(n, format!("{k:?} is not one of the Five Refusals"))
                                })?;
                                if e.cleared.contains(&r) {
                                    return Err(err(n, format!("{k} cleared twice")));
                                }
                                e.cleared.push(r);
                            }
                            e.cleared.sort();
                        }
                        "impact" | "reversible" | "cleared" => return Err(dup(key)),
                        _ => return Err(err(n, format!("unknown effector key {key:?}"))),
                    }
                }
                Section::Constraint(i) => {
                    let (_, _, statement, rule) = &mut constraints[*i];
                    match key {
                        "statement" if statement.is_none() => *statement = Some(value.to_string()),
                        "rule" if rule.is_none() => *rule = Some(parse_rule(n, value)?),
                        "statement" | "rule" => return Err(dup(key)),
                        _ => return Err(err(n, format!("unknown constraint key {key:?}"))),
                    }
                }
            }
        }

        let missing = |k: &str| err(0, format!("missing {k}"));
        let effectors = effectors
            .into_iter()
            .map(|(n, e, seen)| {
                if e.impact.is_nan() {
                    Err(err(n, format!("effector {} has no impact", e.name)))
                } else if !seen[0] {
                    Err(err(
                        n,
                        format!("effector {} does not say if it is reversible", e.name),
                    ))
                } else {
                    Ok(e)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let constraints = constraints
            .into_iter()
            .map(|(n, id, statement, rule)| {
                Ok(Constraint {
                    statement: statement
                        .ok_or_else(|| err(n, format!("constraint {id} has no statement")))?,
                    rule: rule.ok_or_else(|| err(n, format!("constraint {id} has no rule")))?,
                    id,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Charter {
            id: id.ok_or_else(|| missing("id"))?,
            version: cversion.ok_or_else(|| missing("version"))?,
            owner: owner.ok_or_else(|| missing("owner"))?,
            confidence_floor: floor.ok_or_else(|| missing("confidence_floor"))?,
            max_auto_impact: max_auto.ok_or_else(|| missing("max_auto_impact"))?,
            per_window,
            effectors,
            constraints,
        })
    }

    /// The canonical text: what [`Charter::hash`] covers. Comments, blank
    /// lines, key order and spacing in the source do not change it; every
    /// fact the rails read does.
    pub fn canonical(&self) -> String {
        let mut s = format!("{CHARTER_MAGIC} {CHARTER_FORMAT}\n");
        s += &format!("id = {}\n", self.id);
        s += &format!("version = {}\n", self.version);
        s += &format!("owner = {}\n", self.owner);
        s += &format!("confidence_floor = {}\n", self.confidence_floor);
        s += &format!("max_auto_impact = {}\n", self.max_auto_impact);
        if let Some(w) = self.per_window {
            s += &format!("per_window = {} {}\n", w.secs, w.max);
        }
        for e in &self.effectors {
            s += &format!("\n[effector {}]\n", e.name);
            s += &format!("impact = {}\n", e.impact);
            s += &format!("reversible = {}\n", e.reversible);
            let cleared: Vec<&str> = e.cleared.iter().map(|r| r.key()).collect();
            s += &format!("cleared = {}\n", cleared.join(" "));
        }
        for c in &self.constraints {
            s += &format!("\n[constraint {}]\n", c.id);
            s += &format!("statement = {}\n", c.statement);
            s += &format!(
                "rule = {}\n",
                match &c.rule {
                    Rule::RequireHuman(When::Always) => "require_human always".to_string(),
                    Rule::RequireHuman(When::Irreversible) =>
                        "require_human irreversible".to_string(),
                    Rule::MaxImpact(x) => format!("max_impact {x}"),
                    Rule::ForbidIrreversible => "forbid_irreversible".to_string(),
                }
            );
        }
        s
    }

    /// SHA-256 of [`Charter::canonical`]: the exact intent in force, as every
    /// audit entry records it.
    pub fn hash(&self) -> [u8; 32] {
        sha256::digest(self.canonical().as_bytes())
    }

    /// The effector named, if the charter grants it.
    pub fn effector(&self, name: &str) -> Option<&Effector> {
        self.effectors.iter().find(|e| e.name == name)
    }
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(i) => &line[..i],
        None => line,
    }
}

fn parse_rule(line: usize, v: &str) -> Result<Rule, CharterError> {
    let parts: Vec<&str> = v.split_whitespace().collect();
    match parts.as_slice() {
        ["require_human", "always"] => Ok(Rule::RequireHuman(When::Always)),
        ["require_human", "irreversible"] => Ok(Rule::RequireHuman(When::Irreversible)),
        ["max_impact", x] => Ok(Rule::MaxImpact(unit(line, "max_impact", x)?)),
        ["forbid_irreversible"] => Ok(Rule::ForbidIrreversible),
        _ => Err(CharterError {
            line,
            what: format!("unknown rule {v:?}"),
        }),
    }
}

// ---- dry runs ---------------------------------------------------------------

/// A simulation of an effector, registered with the conscience by the
/// binary. It must be pure: it describes what the action would do and does
/// none of it. `Err` is a reason the action would fail.
pub trait DryRun {
    /// Simulate `action`.
    fn dry_run(&self, action: &str) -> Result<String, String>;
}

// ---- the audit chain --------------------------------------------------------

/// What an entry decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// A package for a person to sign.
    Package,
    /// Refused by a named rule.
    Refused,
    /// Sent to the person.
    Escalate,
}

impl Outcome {
    fn key(self) -> &'static str {
        match self {
            Outcome::Package => "package",
            Outcome::Refused => "refused",
            Outcome::Escalate => "escalate",
        }
    }
}

/// One decision, hash-chained onto the one before it. For a package, this
/// entry *is* what the person reads and signs.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// Position in the chain, from 0.
    pub seq: u64,
    /// Unix seconds the decision was made at.
    pub at: u64,
    /// Hash of the charter in force.
    pub charter: [u8; 32],
    /// What was decided.
    pub outcome: Outcome,
    /// The voice's confidence, as proposed.
    pub confidence: f32,
    /// The effector named.
    pub effector: String,
    /// The action, exactly as it will be presented for signature.
    pub action: String,
    /// The refusing rule, the escalation's reasons, or the rails' assent.
    pub detail: String,
    /// What the dry run said, if one ran.
    pub dry_run: Option<String>,
    /// The previous entry's hash; [`GENESIS`] for the first.
    pub prev: [u8; 32],
    /// SHA-256 of this entry's line without its hash, which includes `prev`.
    pub hash: [u8; 32],
}

/// The `prev` of the first entry.
pub const GENESIS: [u8; 32] = [0; 32];

/// The audit file format's version. A newer file is refused.
pub const AUDIT_FORMAT: u32 = 1;
const AUDIT_MAGIC: &str = "kwave-audit";

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

fn unescape(s: &str) -> Option<String> {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        out.push(match it.next()? {
            '\\' => '\\',
            't' => '\t',
            'n' => '\n',
            'r' => '\r',
            _ => return None,
        });
    }
    Some(out)
}

impl Entry {
    /// The line the hash covers: every field but the hash, tab-separated.
    fn body(&self) -> String {
        [
            self.seq.to_string(),
            self.at.to_string(),
            sha256::hex(&self.charter),
            self.outcome.key().to_string(),
            self.confidence.to_string(),
            escape(&self.effector),
            escape(&self.action),
            escape(&self.detail),
            match &self.dry_run {
                None => String::new(),
                Some(d) => format!("+{}", escape(d)),
            },
            sha256::hex(&self.prev),
        ]
        .join("\t")
    }

    fn seal(mut self) -> Entry {
        self.hash = sha256::digest(self.body().as_bytes());
        self
    }

    /// The entry as one line of the audit file.
    pub fn to_line(&self) -> String {
        format!("{}\t{}", self.body(), sha256::hex(&self.hash))
    }

    fn from_line(line: &str) -> Option<Entry> {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() != 11 {
            return None;
        }
        Some(Entry {
            seq: f[0].parse().ok()?,
            at: f[1].parse().ok()?,
            charter: sha256::from_hex(f[2])?,
            outcome: match f[3] {
                "package" => Outcome::Package,
                "refused" => Outcome::Refused,
                "escalate" => Outcome::Escalate,
                _ => return None,
            },
            confidence: f[4].parse().ok()?,
            effector: unescape(f[5])?,
            action: unescape(f[6])?,
            detail: unescape(f[7])?,
            dry_run: match f[8] {
                "" => None,
                d => Some(unescape(d.strip_prefix('+')?)?),
            },
            prev: sha256::from_hex(f[9])?,
            hash: sha256::from_hex(f[10])?,
        })
    }

    /// The verdict this entry records.
    pub fn verdict(&self) -> Verdict {
        match self.outcome {
            Outcome::Package => Verdict::Package {
                action: self.action.clone(),
                audit_hash: self.hash,
            },
            Outcome::Refused => Verdict::Refused {
                rule: self.detail.clone(),
            },
            Outcome::Escalate => Verdict::Escalate {
                reason: self.detail.clone(),
            },
        }
    }
}

/// Why an audit chain was not accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainError {
    /// The entry (by position) where the chain broke, or the line for a
    /// parse failure.
    pub at: usize,
    /// What was wrong.
    pub why: String,
}

impl fmt::Display for ChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "audit chain broken at {}: {}", self.at, self.why)
    }
}

impl std::error::Error for ChainError {}

/// The append-only, hash-chained record of every decision. Altering any past
/// entry breaks every hash after it; [`AuditLog::verify`] finds the first.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AuditLog {
    entries: Vec<Entry>,
}

impl AuditLog {
    /// An empty chain.
    pub fn new() -> AuditLog {
        AuditLog::default()
    }

    /// The entries, oldest first.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Parse the file format and verify the chain. A newer format, a line
    /// that does not parse, or a broken link is an error.
    pub fn parse(text: &str) -> Result<AuditLog, ChainError> {
        let mut lines = text.lines().enumerate().filter(|(_, l)| !l.is_empty());
        let header = match lines.next() {
            None => return Ok(AuditLog::new()),
            Some((_, h)) => h,
        };
        let version = header
            .strip_prefix(AUDIT_MAGIC)
            .map(str::trim)
            .and_then(|v| v.parse::<u32>().ok())
            .ok_or_else(|| ChainError {
                at: 1,
                why: format!("expected `{AUDIT_MAGIC} <version>`"),
            })?;
        if version > AUDIT_FORMAT {
            return Err(ChainError {
                at: 1,
                why: format!("format v{version} is newer than this build reads (v{AUDIT_FORMAT})"),
            });
        }
        let entries = lines
            .map(|(i, l)| {
                Entry::from_line(l).ok_or_else(|| ChainError {
                    at: i + 1,
                    why: "line does not parse as an entry".into(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let log = AuditLog { entries };
        log.verify()?;
        Ok(log)
    }

    /// The header line a new audit file starts with.
    pub fn header() -> String {
        format!("{AUDIT_MAGIC} {AUDIT_FORMAT}")
    }

    /// The whole file.
    pub fn to_text(&self) -> String {
        let mut s = AuditLog::header();
        s.push('\n');
        for e in &self.entries {
            s += &e.to_line();
            s.push('\n');
        }
        s
    }

    /// Re-walk the chain: positions, links, and every hash recomputed.
    pub fn verify(&self) -> Result<(), ChainError> {
        let mut prev = GENESIS;
        for (i, e) in self.entries.iter().enumerate() {
            let broken = |why: String| Err(ChainError { at: i, why });
            if e.seq != i as u64 {
                return broken(format!("seq {} where {i} was expected", e.seq));
            }
            if e.prev != prev {
                return broken("prev does not match the entry before it".into());
            }
            if sha256::digest(e.body().as_bytes()) != e.hash {
                return broken("hash does not match its contents (the entry was altered)".into());
            }
            prev = e.hash;
        }
        Ok(())
    }

    /// The next seq and the hash a new entry must chain onto.
    pub fn head(&self) -> (u64, [u8; 32]) {
        match self.entries.last() {
            None => (0, GENESIS),
            Some(e) => (e.seq + 1, e.hash),
        }
    }

    /// Append an entry decided against this chain's head. An entry decided
    /// against an older head (another decision landed first) is refused, so
    /// two deciders can never fork the chain.
    pub fn append(&mut self, e: Entry) -> Result<(), ChainError> {
        let (seq, prev) = self.head();
        let at = self.entries.len();
        if e.seq != seq || e.prev != prev {
            return Err(ChainError {
                at,
                why: "entry was decided against a stale head".into(),
            });
        }
        if sha256::digest(e.body().as_bytes()) != e.hash {
            return Err(ChainError {
                at,
                why: "entry hash does not match its contents".into(),
            });
        }
        self.entries.push(e);
        Ok(())
    }
}

// ---- the rails --------------------------------------------------------------

/// The conscience: a charter, the audit chain's head and history, the time,
/// and the dry runs the binary registered. Nothing in it can act.
pub struct Conscience {
    charter: Charter,
    charter_hash: [u8; 32],
    head: (u64, [u8; 32]),
    /// When each package in the chain so far was made, for the rate limit.
    packages: Vec<u64>,
    now: u64,
    dry_runs: Vec<(String, Box<dyn DryRun>)>,
}

impl Conscience {
    /// Build against a verified chain. A chain that does not verify is
    /// refused: the rails do not decide against a record they cannot trust.
    pub fn new(charter: Charter, log: &AuditLog, now: u64) -> Result<Conscience, ChainError> {
        log.verify()?;
        Ok(Conscience {
            charter_hash: charter.hash(),
            charter,
            head: log.head(),
            packages: log
                .entries()
                .iter()
                .filter(|e| e.outcome == Outcome::Package)
                .map(|e| e.at)
                .collect(),
            now,
            dry_runs: Vec::new(),
        })
    }

    /// Register the dry run for an effector.
    pub fn with_dry_run(mut self, effector: &str, d: Box<dyn DryRun>) -> Conscience {
        self.dry_runs.retain(|(n, _)| n != effector);
        self.dry_runs.push((effector.to_string(), d));
        self
    }

    /// The charter in force.
    pub fn charter(&self) -> &Charter {
        &self.charter
    }

    /// Decide, and return the sealed audit entry that records it, chained
    /// onto the head this conscience was built with. Pure.
    pub fn entry_for(&self, p: &Proposed) -> Entry {
        let (outcome, detail, dry_run) = self.judge(p);
        Entry {
            seq: self.head.0,
            at: self.now,
            charter: self.charter_hash,
            outcome,
            confidence: p.confidence,
            effector: p.effector.clone(),
            action: p.action.clone(),
            detail,
            dry_run,
            prev: self.head.1,
            hash: [0; 32],
        }
        .seal()
    }

    fn judge(&self, p: &Proposed) -> (Outcome, String, Option<String>) {
        let c = &self.charter;

        // 1. A malformed proposal is refused before anything reads it.
        if !(0.0..=1.0).contains(&p.confidence) {
            return (
                Outcome::Refused,
                format!(
                    "proposal.malformed: confidence {} is not in [0, 1]",
                    p.confidence
                ),
                None,
            );
        }

        // 2. Bounded authority: only what the charter grants.
        let Some(e) = c.effector(&p.effector) else {
            return (
                Outcome::Refused,
                format!(
                    "authority.ungranted: charter {} grants no effector {:?}",
                    c.id, p.effector
                ),
                None,
            );
        };

        let mut escalate: Vec<String> = Vec::new();

        // 3. The Five Refusals, as the person cleared this effector.
        let uncleared: Vec<&str> = Refusal::ALL
            .into_iter()
            .filter(|r| !e.cleared.contains(r))
            .map(|r| r.statement())
            .collect();
        if !uncleared.is_empty() {
            escalate.push(format!(
                "refusals.uncleared: {} is not cleared for \"{}\"; a person must judge this action against them",
                e.name,
                uncleared.join("\", \"")
            ));
        }

        // 4. Confidence floor.
        if p.confidence < c.confidence_floor {
            escalate.push(format!(
                "confidence.floor: {} is below the charter's {}",
                p.confidence, c.confidence_floor
            ));
        }

        // 5. Authority bound on impact.
        if e.impact > c.max_auto_impact {
            escalate.push(format!(
                "authority.impact: {} carries impact {}, above max_auto_impact {}",
                e.name, e.impact, c.max_auto_impact
            ));
        }

        // 6. Hard constraints. A refusal ends the decision.
        for k in &c.constraints {
            match k.rule {
                Rule::ForbidIrreversible if !e.reversible => {
                    return (
                        Outcome::Refused,
                        format!("{}: {} ({} cannot be undone)", k.id, k.statement, e.name),
                        None,
                    );
                }
                Rule::RequireHuman(When::Always) => {
                    escalate.push(format!("{}: {}", k.id, k.statement));
                }
                Rule::RequireHuman(When::Irreversible) if !e.reversible => {
                    escalate.push(format!(
                        "{}: {} ({} cannot be undone)",
                        k.id, k.statement, e.name
                    ));
                }
                Rule::MaxImpact(limit) if e.impact > limit => {
                    escalate.push(format!(
                        "{}: {} (impact {} > {limit})",
                        k.id, k.statement, e.impact
                    ));
                }
                _ => {}
            }
        }

        // 7. Rate limit, from the chain. Packages only: an escalation is the
        // rails declining to act and spends nothing.
        if let Some(w) = c.per_window {
            let since = self.now.saturating_sub(w.secs);
            let n = self
                .packages
                .iter()
                .filter(|&&t| t > since && t <= self.now)
                .count();
            if n >= w.max {
                escalate.push(format!(
                    "rate.window: {n} package(s) in the last {} s, the charter allows {}",
                    w.secs, w.max
                ));
            }
        }

        if !escalate.is_empty() {
            return (Outcome::Escalate, escalate.join("; "), None);
        }

        // 8. Dry run, last: only an action that would package is simulated.
        let Some((_, d)) = self.dry_runs.iter().find(|(n, _)| n == &e.name) else {
            return (
                Outcome::Escalate,
                format!(
                    "dry_run.missing: no dry run is registered for {}; nothing is packaged unsimulated",
                    e.name
                ),
                None,
            );
        };
        match d.dry_run(&p.action) {
            Ok(said) => (
                Outcome::Package,
                "rails: granted, cleared for all five refusals, confident, within authority and rate; dry run ok".into(),
                Some(said),
            ),
            Err(why) => (
                Outcome::Escalate,
                format!("dry_run.failed: {why}"),
                Some(format!("failed: {why}")),
            ),
        }
    }
}

impl Rails for Conscience {
    fn decide(&self, proposed: &Proposed) -> Verdict {
        self.entry_for(proposed).verdict()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHARTER: &str = "\
kwave-charter 1
# a comment, and a blank line, change nothing

id = wave.test
version = 1
owner = a test (label only)
confidence_floor = 0.7
max_auto_impact = 0.3
per_window = 3600 2

[effector wave.remember]
impact = 0.1
reversible = true
cleared = weapons attention power trust abandonment

[effector obc.speak]
impact = 0.2
reversible = false
cleared = weapons power

[effector obc.build]
impact = 0.6
reversible = true
cleared = weapons attention power trust abandonment

[constraint hc.irreversible]
statement = An action that cannot be undone goes to a person.
rule = require_human irreversible
";

    struct Echo;
    impl DryRun for Echo {
        fn dry_run(&self, action: &str) -> Result<String, String> {
            if action.is_empty() {
                Err("nothing to remember".into())
            } else {
                Ok(format!("would write {:?}", action))
            }
        }
    }

    fn charter() -> Charter {
        Charter::parse(CHARTER).expect("test charter parses")
    }

    fn conscience(log: &AuditLog, now: u64) -> Conscience {
        Conscience::new(charter(), log, now)
            .expect("chain verifies")
            .with_dry_run("wave.remember", Box::new(Echo))
    }

    fn p(effector: &str, action: &str, confidence: f32) -> Proposed {
        Proposed {
            effector: effector.into(),
            action: action.into(),
            confidence,
        }
    }

    #[test]
    fn the_charter_parses_and_its_hash_ignores_layout_but_not_facts() {
        let c = charter();
        assert_eq!(c.effectors.len(), 3);
        assert_eq!(c.per_window, Some(Window { secs: 3600, max: 2 }));
        let relaid = CHARTER.replace("\n\n", "\n").replace(" = ", "=");
        assert_eq!(Charter::parse(&relaid).unwrap().hash(), c.hash());
        let changed = CHARTER.replace("confidence_floor = 0.7", "confidence_floor = 0.6");
        assert_ne!(Charter::parse(&changed).unwrap().hash(), c.hash());
        assert_eq!(
            Charter::parse(&c.canonical()).unwrap(),
            c,
            "canonical round-trips"
        );
    }

    #[test]
    fn the_charter_refuses_what_it_does_not_understand() {
        let bad = [
            (
                CHARTER.replace("kwave-charter 1", "kwave-charter 2"),
                "newer",
            ),
            (CHARTER.replace("impact = 0.1", "impact = 1.5"), "[0, 1]"),
            (
                CHARTER.replace("cleared = weapons power", "cleared = weapons guns"),
                "Five Refusals",
            ),
            (CHARTER.replace("owner =", "ownr ="), "unknown key"),
            (
                CHARTER.replace("[effector obc.build]", "[effector wave.remember]"),
                "twice",
            ),
            (CHARTER.replace("impact = 0.2\n", ""), "no impact"),
            (
                CHARTER.replace("rule = require_human irreversible", "rule = be_nice"),
                "unknown rule",
            ),
            (
                CHARTER.replace("max_auto_impact = 0.3\n", ""),
                "missing max_auto_impact",
            ),
        ];
        for (text, want) in bad {
            let e = Charter::parse(&text).expect_err(want);
            assert!(e.to_string().contains(want), "{e} should mention {want:?}");
        }
    }

    #[test]
    fn a_granted_cleared_confident_bounded_action_packages() {
        let log = AuditLog::new();
        let c = conscience(&log, 1000);
        let e = c.entry_for(&p("wave.remember", "the fireflies are finished", 0.9));
        assert_eq!(e.outcome, Outcome::Package, "{}", e.detail);
        assert!(e.dry_run.as_deref().unwrap().contains("fireflies"));
        match c.decide(&p("wave.remember", "the fireflies are finished", 0.9)) {
            Verdict::Package { action, audit_hash } => {
                assert_eq!(action, "the fireflies are finished");
                assert_eq!(audit_hash, e.hash);
            }
            v => panic!("expected a package, got {v:?}"),
        }
    }

    #[test]
    fn decide_is_pure() {
        let log = AuditLog::new();
        let c = conscience(&log, 1000);
        let a = p("obc.speak", "hello", 0.95);
        assert_eq!(c.decide(&a), c.decide(&a));
        assert_eq!(c.entry_for(&a), c.entry_for(&a));
    }

    #[test]
    fn an_ungranted_effector_is_refused_whatever_the_confidence() {
        let c = conscience(&AuditLog::new(), 0);
        match c.decide(&p("wallet.transfer", "everything", 1.0)) {
            Verdict::Refused { rule } => assert!(rule.starts_with("authority.ungranted"), "{rule}"),
            v => panic!("{v:?}"),
        }
    }

    #[test]
    fn a_malformed_confidence_is_refused() {
        let c = conscience(&AuditLog::new(), 0);
        for bad in [f32::NAN, 1.5, -0.1] {
            assert!(matches!(
                c.decide(&p("wave.remember", "x", bad)),
                Verdict::Refused { .. }
            ));
        }
    }

    #[test]
    fn an_effector_not_cleared_for_all_five_refusals_goes_to_a_person() {
        let c = conscience(&AuditLog::new(), 0);
        match c.decide(&p("obc.speak", "hello, city", 0.99)) {
            Verdict::Escalate { reason } => {
                assert!(reason.contains("refusals.uncleared"), "{reason}");
                assert!(reason.contains("sell no attention"));
                assert!(reason.contains("abandon no one"));
                assert!(!reason.contains("build no weapons"), "that one was cleared");
                assert!(
                    reason.contains("hc.irreversible"),
                    "and the constraint also fired"
                );
            }
            v => panic!("{v:?}"),
        }
    }

    #[test]
    fn low_confidence_and_high_impact_escalate() {
        let c = conscience(&AuditLog::new(), 0);
        let low = c.decide(&p("wave.remember", "x", 0.5));
        assert!(
            matches!(&low, Verdict::Escalate { reason } if reason.contains("confidence.floor"))
        );
        let big = c.decide(&p("obc.build", "a tower", 0.99));
        assert!(
            matches!(&big, Verdict::Escalate { reason } if reason.contains("authority.impact"))
        );
    }

    #[test]
    fn forbid_irreversible_refuses_and_outranks_escalation() {
        let text = format!(
            "{CHARTER}\n[constraint hc.never-undoable]\nstatement = Nothing that cannot be undone.\nrule = forbid_irreversible\n"
        );
        let c = Conscience::new(Charter::parse(&text).unwrap(), &AuditLog::new(), 0).unwrap();
        match c.decide(&p("obc.speak", "hello", 0.99)) {
            Verdict::Refused { rule } => assert!(rule.starts_with("hc.never-undoable"), "{rule}"),
            v => panic!("{v:?}"),
        }
    }

    #[test]
    fn no_dry_run_no_package_and_a_failed_dry_run_escalates() {
        let bare = Conscience::new(charter(), &AuditLog::new(), 0).unwrap();
        assert!(matches!(
            bare.decide(&p("wave.remember", "x", 0.9)),
            Verdict::Escalate { reason } if reason.starts_with("dry_run.missing")
        ));
        let c = conscience(&AuditLog::new(), 0);
        let e = c.entry_for(&p("wave.remember", "", 0.9));
        assert_eq!(e.outcome, Outcome::Escalate);
        assert!(e.detail.starts_with("dry_run.failed"));
    }

    #[test]
    fn the_rate_limit_counts_packages_in_the_window_from_the_chain() {
        let mut log = AuditLog::new();
        for (t, conf) in [(100, 0.9), (200, 0.2), (300, 0.9)] {
            let e = conscience(&log, t).entry_for(&p("wave.remember", "x", conf));
            log.append(e).unwrap();
        }
        // Two packages (100, 300) and one escalation (200) in the chain.
        let third = conscience(&log, 400).entry_for(&p("wave.remember", "y", 0.9));
        assert_eq!(third.outcome, Outcome::Escalate);
        assert!(
            third.detail.contains("rate.window: 2 package(s)"),
            "{}",
            third.detail
        );
        // An hour after the first, it has left the window.
        let later = conscience(&log, 3700).entry_for(&p("wave.remember", "y", 0.9));
        assert_eq!(later.outcome, Outcome::Package, "{}", later.detail);
    }

    #[test]
    fn the_chain_round_trips_and_every_tamper_is_found() {
        let mut log = AuditLog::new();
        for (i, (eff, action)) in [
            ("wave.remember", "a line\twith a tab"),
            ("obc.speak", "two\nlines and a \\ backslash"),
            ("nope", "refused"),
        ]
        .into_iter()
        .enumerate()
        {
            let e = conscience(&log, 10 * i as u64).entry_for(&p(eff, action, 0.9));
            log.append(e).unwrap();
        }
        let text = log.to_text();
        assert_eq!(AuditLog::parse(&text).unwrap(), log);

        // Edit one character of a past action: that entry's hash breaks.
        let forged = text.replacen("a line", "b line", 1);
        let err = AuditLog::parse(&forged).unwrap_err();
        assert_eq!(err.at, 0, "{err}");
        assert!(err.why.contains("altered"));

        // Drop an entry: the link after it breaks.
        let mut lines: Vec<&str> = text.lines().collect();
        lines.remove(2);
        let err = AuditLog::parse(&lines.join("\n")).unwrap_err();
        assert!(err.why.contains("seq"), "{err}");

        // Turn an escalation into a package and re-hash it honestly: the next
        // entry's prev no longer matches, so the forgery still shows.
        let mut entries = log.entries().to_vec();
        entries[1].outcome = Outcome::Package;
        entries[1] = entries[1].clone().seal();
        let err = AuditLog { entries }.verify().unwrap_err();
        assert_eq!(err.at, 2);
    }

    #[test]
    fn a_tampered_chain_is_refused_before_anything_is_decided() {
        let mut log = AuditLog::new();
        log.append(conscience(&log, 0).entry_for(&p("wave.remember", "x", 0.9)))
            .unwrap();
        let mut entries = log.entries().to_vec();
        entries[0].action = "something else".into();
        assert!(Conscience::new(charter(), &AuditLog { entries }, 1).is_err());
    }

    #[test]
    fn an_entry_decided_against_a_stale_head_is_refused() {
        let mut log = AuditLog::new();
        let c = conscience(&log, 0);
        let first = c.entry_for(&p("wave.remember", "one", 0.9));
        let second = c.entry_for(&p("wave.remember", "two", 0.9));
        log.append(first).unwrap();
        let err = log.append(second).unwrap_err();
        assert!(err.why.contains("stale"));
    }

    #[test]
    fn a_newer_audit_file_is_refused() {
        let err = AuditLog::parse("kwave-audit 9\n").unwrap_err();
        assert!(err.why.contains("newer"));
    }

    #[test]
    fn the_shipped_example_charter_parses_and_packages_only_remember() {
        let c = Charter::parse(include_str!("../charters/wave.example.kwc"))
            .expect("charters/wave.example.kwc parses");
        let rails = Conscience::new(c, &AuditLog::new(), 0)
            .unwrap()
            .with_dry_run("wave.remember", Box::new(Echo));
        assert!(matches!(
            rails.decide(&p("wave.remember", "a fact", 0.9)),
            Verdict::Package { .. }
        ));
        assert!(matches!(
            rails.decide(&p("obc.speak", "hello", 0.99)),
            Verdict::Escalate { .. }
        ));
    }

    #[test]
    fn every_entry_names_the_charter_it_was_decided_under() {
        let log = AuditLog::new();
        let e = conscience(&log, 0).entry_for(&p("nope", "x", 0.9));
        assert_eq!(e.charter, charter().hash());
    }
}
