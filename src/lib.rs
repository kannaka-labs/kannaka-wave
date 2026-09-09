//! Kannaka Wave — an AI built from the ground up on the Holographic Resonance
//! Medium, with the language model as an organ rather than a layer.
//!
//! Read `docs/adr/ADR-0001-kannaka-wave.md` first, then
//! `docs/archaeology/README.md` for where every idea here came from and what was
//! measured about it. This crate commits to **interfaces** before implementations:
//! the organs are traits (four in ADR-0001, a fifth in ADR-0002). E-001 decided
//! on 2026-09-09 that the waves lose: the substrate is a plain vector store with
//! the voice's encoder, atomic facets and a stated forgetting policy. The chiral
//! number system that was here is in `docs/lineage/`, out of the build.
//!
//! No dependencies yet. That is deliberate: the first thing this crate must be
//! able to say is what it is, and it should be able to say it with `cargo test`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// A stable identifier for anything that persists. Never a string chosen by a
/// wire peer (ADR-0039): the substrate assigns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(pub u128);

/// An embedding in the voice's own space. The hash codebook does not come over.
#[derive(Debug, Clone, PartialEq)]
pub struct Vector(pub Vec<f32>);

/// One atomic fact, as written. A compound experience is decomposed into these
/// at write time; the compound survives only as a resolve-only parent.
#[derive(Debug, Clone, PartialEq)]
pub struct Facet {
    /// The text of the fact.
    pub text: String,
    /// The compound it came from, if any.
    pub parent: Option<Id>,
}

/// A recalled facet with the numbers that justify it, named for what they are.
#[derive(Debug, Clone, PartialEq)]
pub struct Recalled {
    /// Which facet.
    pub id: Id,
    /// Cosine in the encoder's space, in `[0, 1]`.
    pub similarity: f32,
    /// Constructive-interference magnitude, **unbounded**. Absent from a
    /// substrate that has no waves, which is what E-001 decided the substrate
    /// is; the field stays so the record of the arm-W numbers still types.
    pub resonance: Option<f32>,
}

/// A retention policy, per content class, stated rather than implied.
#[derive(Debug, Clone, PartialEq)]
pub struct Retention {
    /// Content prefix this row governs (e.g. `audio:heard`).
    pub class: String,
    /// Maximum kept; oldest-and-least-recalled go first. `None` = uncapped.
    pub cap: Option<usize>,
    /// Time to live in days. `None` = no expiry.
    pub ttl_days: Option<u32>,
}

/// What one dream did, from the medium's point of view. A dream that reports
/// zero dissolved when the policy says some should have dissolved is a broken
/// detector, and E-001 checks for exactly that before it trusts anything.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DreamReport {
    /// Facets consolidated (strengthened or merged).
    pub consolidated: usize,
    /// Facets the voice proposed across clusters (ADR-0005's hallucination,
    /// placed where it belongs).
    pub proposed: usize,
    /// Facets that dissolved under the retention policy.
    pub dissolved: usize,
}

/// The encoder: text to the voice's space. Deterministic for a given model.
pub trait Encoder {
    /// Encode one text.
    fn encode(&self, text: &str) -> Vector;
    /// Dimensionality of every vector this encoder produces.
    fn dims(&self) -> usize;
}

/// Organ 1. The substrate decides what persists. Both E-001 arms implement this.
pub trait Substrate {
    /// Absorb one facet, already encoded. Returns the substrate-assigned id.
    fn absorb(&mut self, facet: Facet, vector: Vector) -> Id;
    /// Recall against a question, never against a prompt.
    fn recall(&self, question: &Vector, top_k: usize) -> Vec<Recalled>;
    /// One dream: consolidate, let the voice propose, forget per policy.
    fn dream(&mut self, policy: &[Retention]) -> DreamReport;
    /// How many facets are held right now.
    fn len(&self) -> usize;
    /// Whether nothing is held.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Organ 2. The voice is stateless; it reads the substrate through `recall` and
/// enters it only through `dream`.
pub trait Voice {
    /// Answer in her voice, given the facets recall returned. Never given the
    /// raw prompt as a query; the caller has already reduced it to a question.
    fn speak(&self, question: &str, facets: &[Recalled]) -> String;
    /// Propose a connection between distant facets, for the dream to dispose of.
    fn propose(&self, parents: &[Facet]) -> Option<Facet>;
}

/// A proposed action, before the conscience has seen it.
#[derive(Debug, Clone, PartialEq)]
pub struct Proposed {
    /// What the voice wants to do, as a serialisable description.
    pub action: String,
    /// The voice's own confidence, in `[0, 1]`.
    pub confidence: f32,
}

/// What the rails decided. There is no `Execute` variant here: the rails emit a
/// package for a person to sign, and the effector is somewhere else.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// Within the charter and bounds: a human-signable package, hashed.
    Package {
        /// The action exactly as it will be presented for signature.
        action: String,
        /// Hash of this entry chained onto the previous audit entry.
        audit_hash: [u8; 32],
    },
    /// Refused, with the rule that refused it, by name.
    Refused {
        /// The charter constraint or rail that said no.
        rule: String,
    },
    /// Ambiguous: escalate to the person. The default.
    Escalate {
        /// What was ambiguous, in one sentence a person can act on.
        reason: String,
    },
}

/// Organ 3. The conscience. The reasoner is never the arbiter.
pub trait Rails {
    /// Decide. Must be pure in the proposed action and the charter it was built
    /// with; two calls with the same inputs return the same verdict.
    fn decide(&self, proposed: &Proposed) -> Verdict;
}

/// Organ 5 (ADR-0002). The world: a latent predictor over the same space the
/// encoder produces and recall ranks in. It never writes a row; its only path
/// into the substrate is the salience of a row the substrate was about to
/// write anyway, and its only path into a dream is a rollout the rails have
/// already decided on, action by action.
pub trait World {
    /// Observe the world's current state, `Z(t)`, in the encoder's space.
    fn observe(&mut self, state: &Vector);
    /// Predict the next state given the current one and, optionally, an
    /// action the voice proposed. `None` is "the world proceeds without me".
    fn predict(&self, state: &Vector, action: Option<&str>) -> Vector;
    /// The raw drive for surprise: how far the world landed from the
    /// prediction, in the encoder's space. Zero when exact, never negative.
    /// This is the *distance*; the salience signal is the fast-minus-slow
    /// response to it (ADR-0040), so a constant stream yields zero salience
    /// even though each step has a distance.
    fn surprise(&self, predicted: &Vector, observed: &Vector) -> f32 {
        debug_assert_eq!(predicted.0.len(), observed.0.len());
        predicted
            .0
            .iter()
            .zip(&observed.0)
            .map(|(p, o)| (p - o) * (p - o))
            .sum::<f32>()
            .sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recalled_row_names_its_numbers_honestly() {
        let r = Recalled {
            id: Id(1),
            similarity: 0.76,
            resonance: None,
        };
        assert!(r.similarity <= 1.0);
        assert!(
            r.resonance.is_none(),
            "a vector arm has no resonance and says so"
        );
    }

    /// A world in which nothing ever changes. Its prediction is always right,
    /// so its surprise is always zero; that is the property the salience path
    /// depends on, and the default `surprise` must deliver it exactly.
    struct StillWorld;
    impl World for StillWorld {
        fn observe(&mut self, _state: &Vector) {}
        fn predict(&self, state: &Vector, _action: Option<&str>) -> Vector {
            state.clone()
        }
    }

    #[test]
    fn a_perfect_prediction_has_zero_surprise_and_a_wrong_one_does_not() {
        let w = StillWorld;
        let z = Vector(vec![0.5, -0.25, 1.0]);
        let z_hat = w.predict(&z, None);
        assert_eq!(w.surprise(&z_hat, &z), 0.0, "exact prediction: no surprise");
        let moved = Vector(vec![0.5, -0.25, 0.0]);
        assert!((w.surprise(&z_hat, &moved) - 1.0).abs() < 1e-6);
        assert!(w.surprise(&moved, &z_hat) >= 0.0, "never negative");
    }

    #[test]
    fn a_verdict_has_no_way_to_execute() {
        // The type is the invariant: there is no variant that acts.
        let v = Verdict::Escalate {
            reason: "nothing said yes".into(),
        };
        match v {
            Verdict::Package { .. } | Verdict::Refused { .. } | Verdict::Escalate { .. } => {}
        }
    }
}
