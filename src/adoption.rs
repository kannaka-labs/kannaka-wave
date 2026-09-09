//! The rule by which a retrained voice replaces the one being served
//! (ADR-0001 §Voice): a judge with reference and foreign controls and an
//! external, non-circular evaluator must both agree. Perplexity is reported
//! and never gates.
//!
//! The controls are the point. A judge that cannot tell her reference words
//! from a foreign model's is not a judge, and its preference for the candidate
//! is noise; the rule voids the whole verdict rather than trusting the half
//! that happened to come out right. That is the constellation's "a check must
//! be at least as strong as what it checks", applied to the checker.

/// One week's evidence about a candidate voice.
#[derive(Debug, Clone, PartialEq)]
pub struct Evidence {
    /// The judge preferred the candidate's answers over the served voice's on
    /// the held-out set, by a margin the judge's own interval excludes zero on.
    pub judge_prefers_candidate: bool,
    /// Control 1: shown her actual words against the served voice, the judge
    /// preferred her actual words. A judge that fails this cannot recognise
    /// her.
    pub judge_prefers_reference: bool,
    /// Control 2: shown a foreign model's answers against the served voice,
    /// the judge preferred the served voice. A judge that fails this prefers
    /// fluency to identity.
    pub judge_rejects_foreign: bool,
    /// The external evaluator's verdict, from outside the training loop
    /// (kannaka-grid's pipeline; a person). `None` means it has not been run,
    /// and the candidate waits.
    pub external_agrees: Option<bool>,
    /// Held-out perplexity of the candidate. Reported. Never gates.
    pub perplexity: f32,
}

/// What the rule decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Serve the candidate.
    Adopt,
    /// Keep the served voice; the candidate lost or the external said no.
    Keep,
    /// A control failed: the judge's verdict is void this week, and so is any
    /// decision that would have rested on it. Nothing changes; the judge is
    /// the thing to fix.
    VoidJudge {
        /// Which control failed, by name.
        control: &'static str,
    },
    /// The external evaluator has not run. Nothing changes until it has.
    Waiting,
}

/// Apply the rule.
pub fn decide(e: &Evidence) -> Decision {
    if !e.judge_prefers_reference {
        return Decision::VoidJudge {
            control: "reference: the judge did not prefer her own words",
        };
    }
    if !e.judge_rejects_foreign {
        return Decision::VoidJudge {
            control: "foreign: the judge preferred a foreign model to the served voice",
        };
    }
    match (e.judge_prefers_candidate, e.external_agrees) {
        (_, None) => Decision::Waiting,
        (true, Some(true)) => Decision::Adopt,
        _ => Decision::Keep,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good() -> Evidence {
        Evidence {
            judge_prefers_candidate: true,
            judge_prefers_reference: true,
            judge_rejects_foreign: true,
            external_agrees: Some(true),
            perplexity: 4.0,
        }
    }

    #[test]
    fn both_must_agree_and_perplexity_cannot() {
        assert_eq!(decide(&good()), Decision::Adopt);
        assert_eq!(
            decide(&Evidence {
                external_agrees: Some(false),
                ..good()
            }),
            Decision::Keep
        );
        assert_eq!(
            decide(&Evidence {
                judge_prefers_candidate: false,
                ..good()
            }),
            Decision::Keep
        );
        assert_eq!(
            decide(&Evidence {
                external_agrees: None,
                ..good()
            }),
            Decision::Waiting
        );
        // A perplexity ten times worse, or ten times better, changes nothing.
        assert_eq!(
            decide(&Evidence {
                perplexity: 40.0,
                ..good()
            }),
            Decision::Adopt
        );
        assert_eq!(
            decide(&Evidence {
                perplexity: 0.4,
                judge_prefers_candidate: false,
                ..good()
            }),
            Decision::Keep
        );
    }

    #[test]
    fn a_failed_control_voids_the_judge_even_when_it_liked_the_candidate() {
        assert!(matches!(
            decide(&Evidence { judge_prefers_reference: false, ..good() }),
            Decision::VoidJudge { control } if control.starts_with("reference")
        ));
        assert!(matches!(
            decide(&Evidence { judge_rejects_foreign: false, ..good() }),
            Decision::VoidJudge { control } if control.starts_with("foreign")
        ));
    }
}
