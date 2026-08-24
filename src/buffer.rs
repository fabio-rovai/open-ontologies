//! Build-time modelling buffer.
//!
//! Keeps proprietary material out of the payloads sent to a remote model while
//! an ontology is being authored. This is a *build-time* control and has
//! nothing to do with runtime access control over queries.
//!
//! Full design, including the residual risks this does **not** address, is in
//! `docs/superpowers/specs/2026-08-09-modelling-buffer-design.md`.
//!
//! The governing principle is that ontology construction needs the *shape* of a
//! domain, not its contents: `Candidate hasTarget Protein` is modellable from a
//! schema without knowing which candidates or which proteins exist.

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::state::StateDb;

/// What happens to a term at the boundary.
///
/// Every fallback in this module moves toward disclosing less. Unclassified
/// terms become [`Disposition::Strip`], and a `Surrogate` with no available
/// substitute degrades to [`Disposition::Tokenise`], never to
/// [`Disposition::Pass`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Disposition {
    /// Generic domain vocabulary. Leaves unchanged.
    ///
    /// Only ever assigned from an explicit allowlist, never as a default:
    /// "Protein" is nobody's IP, and substituting it would cost the model its
    /// ability to name classes meaningfully while protecting nothing.
    Pass,
    /// Never leaves. The default for anything unrecognised.
    Strip,
    /// Replaced by an opaque token. Use where the model needs identity and
    /// equality but not meaning: keys, accessions, internal codenames.
    Tokenise,
    /// Replaced by a plausible same-class substitute. Use where the model's
    /// domain knowledge has to fire for the modelling to be any good.
    ///
    /// Surrogates carry a hazard tokens do not: they lie plausibly. See
    /// [`contamination_check`].
    Surrogate,
}

/// What kind of thing a term is. Constrains its default disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TermKind {
    /// Table, column, class or property name.
    SchemaName,
    /// Human-readable label or comment.
    Label,
    /// A row cell or literal. Never required to build a TBox.
    Instance,
    /// Key, accession, UUID, codename.
    Identifier,
}

/// A term extracted from a source, awaiting a disposition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Term {
    pub kind: TermKind,
    pub value: String,
    /// Optional class hint used to pick a surrogate, e.g. `"protein"`.
    #[serde(default)]
    pub class_hint: Option<String>,
}

impl Term {
    pub fn new(kind: TermKind, value: impl Into<String>) -> Self {
        Self {
            kind,
            value: value.into(),
            class_hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.class_hint = Some(hint.into());
        self
    }
}

/// A disposition applied to a term, and what (if anything) replaces it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub term: Term,
    pub disposition: Disposition,
    /// What actually goes out. `None` for [`Disposition::Strip`].
    pub replacement: Option<String>,
    /// Why this disposition was chosen. Shown at the review gate, because a
    /// human approving a payload needs the reason, not just the verdict.
    pub rationale: String,
}

impl Decision {
    /// Does this decision put anything on the wire?
    pub fn emits(&self) -> bool {
        self.replacement.is_some()
    }
}

/// Rules deciding what happens to each term.
///
/// Deliberately a rules-and-dictionary classifier rather than a model. It has
/// to see real terms, so it must run locally; and a human is going to approve
/// its output, which requires that its decisions be explainable.
#[derive(Debug, Clone, Default)]
pub struct Classifier {
    /// Terms that may leave unchanged. Compared case-insensitively.
    generic_vocabulary: HashSet<String>,
    /// Substitute pools by class hint, e.g. `"protein" -> ["EGFR", "TP53"]`.
    surrogate_pools: HashMap<String, Vec<String>>,
}

impl Classifier {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add terms that are safe to send unchanged.
    pub fn allow_generic<I, S>(mut self, terms: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for t in terms {
            self.generic_vocabulary.insert(t.into().to_lowercase());
        }
        self
    }

    /// Register substitutes for a class hint.
    pub fn with_surrogates<I, S>(mut self, hint: impl Into<String>, pool: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.surrogate_pools.insert(
            hint.into().to_lowercase(),
            pool.into_iter().map(Into::into).collect(),
        );
        self
    }

    pub fn is_generic(&self, value: &str) -> bool {
        self.generic_vocabulary.contains(&value.to_lowercase())
    }

    /// Choose a disposition, without yet computing the replacement.
    ///
    /// Order matters. Instances are stripped before anything else can rescue
    /// them, so a row value that happens to collide with a generic vocabulary
    /// term is still stripped.
    pub fn classify(&self, term: &Term) -> (Disposition, String) {
        // Instances never leave, whatever they look like.
        if term.kind == TermKind::Instance {
            return (
                Disposition::Strip,
                "instance data is never required to build a TBox".to_string(),
            );
        }

        // Identifiers carry no modellable meaning, only identity.
        if term.kind == TermKind::Identifier {
            return (
                Disposition::Tokenise,
                "identifier: the model needs equality, not meaning".to_string(),
            );
        }

        if self.is_generic(&term.value) {
            return (
                Disposition::Pass,
                "on the generic-vocabulary allowlist".to_string(),
            );
        }

        // A schema name with a usable class hint is worth surrogating, because
        // the model's domain knowledge is what makes the resulting hierarchy
        // good rather than merely well-formed.
        if term.kind == TermKind::SchemaName {
            if let Some(hint) = &term.class_hint
                && self.pool_for(hint).is_some_and(|p| !p.is_empty())
            {
                return (
                    Disposition::Surrogate,
                    format!("non-generic schema name with a '{hint}' substitute available"),
                );
            }
            return (
                Disposition::Tokenise,
                "non-generic schema name with no substitute available".to_string(),
            );
        }

        (
            Disposition::Strip,
            "unrecognised: defaulting to strip".to_string(),
        )
    }

    fn pool_for(&self, hint: &str) -> Option<&Vec<String>> {
        self.surrogate_pools.get(&hint.to_lowercase())
    }
}

/// Session-scoped store mapping what left the boundary back to what it stood
/// for. Never crosses the boundary itself.
pub struct Vault {
    db: StateDb,
    session_id: String,
    salt: String,
}

impl Vault {
    /// Open (or create) the vault for `session_id`.
    ///
    /// The salt is per session, so a given term maps to the same surrogate
    /// throughout one modelling run, letting the model join and group, while
    /// not accumulating a stable corpus at the provider across runs.
    pub fn open(db: StateDb, session_id: impl Into<String>) -> Result<Self> {
        let session_id = session_id.into();
        let salt = {
            let conn = db.conn();
            let existing: Option<String> = conn
                .query_row(
                    "SELECT salt FROM buffer_sessions WHERE session_id = ?1",
                    [&session_id],
                    |r| r.get(0),
                )
                .ok();
            match existing {
                Some(s) => s,
                None => {
                    // Derived from the session id plus the database's own
                    // rowid clock rather than a RNG, so the vault is
                    // reproducible for a given database without pulling in a
                    // randomness dependency. It is a domain separator, not a
                    // secret: confidentiality here rests on the mapping never
                    // leaving, not on the salt being unguessable.
                    let seed: i64 = conn
                        .query_row("SELECT COUNT(*) FROM buffer_sessions", [], |r| r.get(0))
                        .unwrap_or(0);
                    let salt = hex8(&format!("{session_id}:{seed}"));
                    conn.execute(
                        "INSERT INTO buffer_sessions (session_id, salt) VALUES (?1, ?2)",
                        [&session_id, &salt],
                    )?;
                    salt
                }
            }
        };
        Ok(Self {
            db,
            session_id,
            salt,
        })
    }

    /// Deterministic within the session, distinct across sessions.
    fn derive(&self, original: &str, prefix: &str) -> String {
        format!(
            "{prefix}_{}",
            hex8(&format!("{}:{}", self.salt, original))
        )
    }

    fn record(&self, surrogate: &str, original: &str, disposition: Disposition) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "INSERT OR IGNORE INTO buffer_vault (session_id, surrogate, original, disposition) \
             VALUES (?1, ?2, ?3, ?4)",
            [
                self.session_id.as_str(),
                surrogate,
                original,
                match disposition {
                    Disposition::Tokenise => "tokenise",
                    Disposition::Surrogate => "surrogate",
                    Disposition::Pass => "pass",
                    Disposition::Strip => "strip",
                },
            ],
        )?;
        Ok(())
    }

    /// Opaque replacement. Carries no meaning, so nothing can be inferred from
    /// it beyond equality with other occurrences.
    pub fn tokenise(&self, original: &str) -> Result<String> {
        let token = self.derive(original, "ENT");
        self.record(&token, original, Disposition::Tokenise)?;
        Ok(token)
    }

    /// Plausible same-class replacement drawn from `pool`.
    ///
    /// Selection is deterministic in the salted hash, so the same term gets the
    /// same substitute all run. Returns `None` when the pool is empty; callers
    /// must then fall back to [`Vault::tokenise`], never to passing the term
    /// through.
    pub fn surrogate(&self, original: &str, pool: &[String]) -> Result<Option<String>> {
        if pool.is_empty() {
            return Ok(None);
        }
        let digest = hex8(&format!("{}:{}", self.salt, original));
        let idx = usize::from_str_radix(&digest, 16).unwrap_or(0) % pool.len();
        let chosen = pool[idx].clone();
        self.record(&chosen, original, Disposition::Surrogate)?;
        Ok(Some(chosen))
    }

    /// What did this surrogate stand for?
    pub fn resolve(&self, surrogate: &str) -> Result<Option<String>> {
        let conn = self.db.conn();
        Ok(conn
            .query_row(
                "SELECT original FROM buffer_vault WHERE session_id = ?1 AND surrogate = ?2",
                [self.session_id.as_str(), surrogate],
                |r| r.get(0),
            )
            .ok())
    }

    /// Every substitution made this session, longest surrogate first.
    ///
    /// The ordering is what makes [`Vault::desubstitute`] correct: replacing
    /// longer surrogates first stops a short one that is a prefix of a longer
    /// one from corrupting it.
    pub fn substitutions(&self) -> Result<Vec<(String, String)>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT surrogate, original FROM buffer_vault WHERE session_id = ?1 \
             ORDER BY length(surrogate) DESC",
        )?;
        let rows = stmt
            .query_map([self.session_id.as_str()], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Map a returned artefact back to real vocabulary.
    pub fn desubstitute(&self, text: &str) -> Result<String> {
        let mut out = text.to_string();
        for (surrogate, original) in self.substitutions()? {
            out = out.replace(&surrogate, &original);
        }
        Ok(out)
    }
}

/// A payload ready for human review, and the decisions behind it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferedPayload {
    pub decisions: Vec<Decision>,
}

impl BufferedPayload {
    /// The terms that actually leave.
    pub fn emitted(&self) -> Vec<&str> {
        self.decisions
            .iter()
            .filter_map(|d| d.replacement.as_deref())
            .collect()
    }

    /// Original values that must not appear in anything sent outward. The
    /// canary suite asserts against exactly this set.
    pub fn withheld(&self) -> Vec<&str> {
        self.decisions
            .iter()
            .filter(|d| d.disposition != Disposition::Pass)
            .map(|d| d.term.value.as_str())
            .collect()
    }

    /// Human-readable rendering for the review gate.
    ///
    /// This is the load-bearing control. A classifier's recall is arguable; a
    /// log of exactly what left, approved by a person, is not.
    pub fn render_for_review(&self) -> String {
        let mut out = String::from("Modelling buffer: payload for review\n\n");
        out.push_str(&format!("{:<28} {:<10} {:<28} {}\n", "TERM", "ACTION", "SENT AS", "WHY"));
        out.push_str(&"-".repeat(100));
        out.push('\n');
        for d in &self.decisions {
            let action = match d.disposition {
                Disposition::Pass => "pass",
                Disposition::Strip => "STRIP",
                Disposition::Tokenise => "tokenise",
                Disposition::Surrogate => "surrogate",
            };
            let sent = d.replacement.as_deref().unwrap_or("(nothing)");
            out.push_str(&format!(
                "{:<28} {:<10} {:<28} {}\n",
                truncate(&d.term.value, 27),
                action,
                truncate(sent, 27),
                d.rationale
            ));
        }
        let emitted = self.decisions.iter().filter(|d| d.emits()).count();
        out.push_str(&format!(
            "\n{} terms, {} leave, {} withheld\n",
            self.decisions.len(),
            emitted,
            self.decisions.len() - emitted
        ));
        out
    }
}

/// Apply `classifier` to `terms`, recording substitutions in `vault`.
pub fn buffer(
    classifier: &Classifier,
    vault: &Vault,
    terms: &[Term],
) -> Result<BufferedPayload> {
    let mut decisions = Vec::with_capacity(terms.len());
    for term in terms {
        let (disposition, mut rationale) = classifier.classify(term);
        let (disposition, replacement) = match disposition {
            Disposition::Pass => (disposition, Some(term.value.clone())),
            Disposition::Strip => (disposition, None),
            Disposition::Tokenise => (disposition, Some(vault.tokenise(&term.value)?)),
            Disposition::Surrogate => {
                let pool = term
                    .class_hint
                    .as_deref()
                    .and_then(|h| classifier.pool_for(h));
                match pool.and_then(|p| vault.surrogate(&term.value, p).transpose()) {
                    Some(chosen) => (Disposition::Surrogate, Some(chosen?)),
                    None => {
                        // Fail safe: degrade to an opaque token, never to Pass.
                        rationale =
                            "surrogate unavailable; degraded to tokenise".to_string();
                        (Disposition::Tokenise, Some(vault.tokenise(&term.value)?))
                    }
                }
            }
        };
        decisions.push(Decision {
            term: term.clone(),
            disposition,
            replacement,
            rationale,
        });
    }
    Ok(BufferedPayload { decisions })
}

/// Predicates a surrogate may legitimately appear with.
///
/// A surrogate may inform **structure and typing**. It may never contribute
/// **content**. Substitute a public protein for a proprietary one and the model
/// will happily assert facts true of the public one; after de-substitution
/// those are asserted about your term, silently and confidently.
const STRUCTURAL_PREDICATES: &[&str] = &[
    "rdf:type",
    "rdfs:subClassOf",
    "rdfs:subPropertyOf",
    "rdfs:domain",
    "rdfs:range",
    "owl:disjointWith",
    "owl:equivalentClass",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
    "http://www.w3.org/2000/01/rdf-schema#subClassOf",
    "http://www.w3.org/2000/01/rdf-schema#subPropertyOf",
    "http://www.w3.org/2000/01/rdf-schema#domain",
    "http://www.w3.org/2000/01/rdf-schema#range",
    "http://www.w3.org/2002/07/owl#disjointWith",
    "http://www.w3.org/2002/07/owl#equivalentClass",
];

/// A triple that mentions a surrogate in a content-bearing position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contamination {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub surrogate: String,
}

/// Outcome of the contamination pass.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContaminationReport {
    /// Triples that must be dropped rather than de-substituted.
    pub rejected: Vec<Contamination>,
    pub accepted_count: usize,
}

impl ContaminationReport {
    pub fn is_clean(&self) -> bool {
        self.rejected.is_empty()
    }
}

/// Reject axioms whose truth depends on a surrogate's identity.
///
/// Without this pass surrogates are a correctness bug wearing a security
/// control's clothing: the returned ontology looks fine and asserts things
/// about your terms that were only ever true of their substitutes.
pub fn contamination_check(
    triples: &[(String, String, String)],
    surrogates: &HashSet<String>,
) -> ContaminationReport {
    let mut report = ContaminationReport::default();
    for (s, p, o) in triples {
        let structural = STRUCTURAL_PREDICATES.contains(&p.as_str());
        // A surrogate in subject position with a structural predicate is the
        // permitted case: it places the term in the hierarchy. Anywhere else,
        // the assertion depends on which entity the surrogate actually is.
        let offending = if structural {
            None
        } else {
            [s, o].into_iter().find(|v| surrogates.contains(*v))
        };
        match offending {
            Some(sur) => report.rejected.push(Contamination {
                subject: s.clone(),
                predicate: p.clone(),
                object: o.clone(),
                surrogate: sur.clone(),
            }),
            None => report.accepted_count += 1,
        }
    }
    report
}

fn hex8(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    digest[..4].iter().map(|b| format!("{b:02x}")).collect()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}
