//! Planner stub (#45) — PDDL emission from Dynamics action schemas.
//!
//! Compiles registered [`crate::dynamics::ActionSchema`]s into a PDDL domain
//! and emits a problem instance from the loaded graph plus a Turtle goal slice.
//! The actual planning is delegated to an external solver (Fast Downward) per
//! the LLM-Modulo convention: the server only exposes the PDDL compilation +
//! plan validation; the orchestrator runs the solver and feeds steps back.
//!
//! ## Bounded scope (v0.4 stub)
//!
//! - Single predicate `(triple ?s ?p ?o)` with one typed sort `iri`.
//! - Parameter slots from `ActionSchema::parameters` become typed PDDL
//!   parameters.
//! - Preconditions of the form `ASK { <{a}> <pred> <{b}> }` are translated to
//!   `(triple ?a "pred-as-constant" ?b)`. Free-form SPARQL is preserved as a
//!   PDDL comment so the audit trail is explicit about the lossy translation.
//! - `AddTriple` / `AddClass` effects compile to positive `triple` facts.
//! - `RemoveTriple` compiles to negated `triple` facts via STRIPS delete.
//!
//! ## Honest deferral
//!
//! Real OWL → PDDL compilation requires careful handling of existentials,
//! cardinality restrictions, and open-world semantics. The Borgwardt KR 2025
//! paper is the anchor for the rigorous version; this stub is a sand-table.

use crate::dynamics::{ActionSchema, EffectSpec};

/// Result of compiling a domain + problem.
#[derive(Clone, Debug)]
pub struct CompiledPddl {
    pub domain: String,
    pub problem: String,
    /// Translation notes: each entry is a precondition or effect that the
    /// stub couldn't fully encode and that was preserved as a PDDL comment.
    pub translation_notes: Vec<String>,
}

/// Sanitise an IRI or label into a PDDL-safe identifier (no angle brackets,
/// slashes, colons; lower-case alphanumeric + `_`).
fn sanitise(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

/// Try to extract `(subject_param, predicate_iri, object_param)` from an
/// SPARQL `ASK { <{a}> <pred> <{b}> }` precondition. Returns None for
/// anything more complex.
fn try_extract_ask_triple(precond: &str) -> Option<(String, String, String)> {
    let trimmed = precond.trim();
    if !trimmed.to_uppercase().starts_with("ASK") {
        return None;
    }
    let open = trimmed.find('{')?;
    let close = trimmed.rfind('}')?;
    if close <= open {
        return None;
    }
    let body = trimmed[open + 1..close].trim();
    // Expect three IRIs / placeholders separated by whitespace, ending in `.` or not.
    let stripped = body.trim_end_matches('.').trim();
    let parts: Vec<&str> = stripped.split_whitespace().collect();
    if parts.len() != 3 {
        return None;
    }
    Some((parts[0].to_string(), parts[1].to_string(), parts[2].to_string()))
}

/// Translate a triple position (placeholder `<{x}>`, full IRI `<...>`, or bare
/// `{x}`) into a PDDL term. Placeholders become `?x`; full IRIs become a
/// stable constant identifier.
fn position_to_pddl_term(pos: &str, notes: &mut Vec<String>) -> String {
    let t = pos.trim();
    // <{x}> or <{x}>
    if let Some(inner) = t.strip_prefix("<{").and_then(|s| s.strip_suffix("}>")) {
        return format!("?{}", inner);
    }
    if let Some(inner) = t.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        return format!("?{}", inner);
    }
    if let Some(inner) = t.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
        return sanitise(inner);
    }
    notes.push(format!("untranslated term: {}", t));
    sanitise(t)
}

fn effect_to_pddl(effect: &EffectSpec, notes: &mut Vec<String>) -> (bool, String) {
    // Returns (positive?, pddl_atom).
    match effect {
        EffectSpec::AddTriple { subject, predicate, object } => {
            let s = position_to_pddl_term(&format!("<{}>", subject.trim_matches(|c| c == '<' || c == '>')), notes);
            let p = position_to_pddl_term(&format!("<{}>", predicate.trim_matches(|c| c == '<' || c == '>')), notes);
            let o = position_to_pddl_term(&format!("<{}>", object.trim_matches(|c| c == '<' || c == '>')), notes);
            (true, format!("(triple {} {} {})", s, p, o))
        }
        EffectSpec::RemoveTriple { subject, predicate, object } => {
            let s = position_to_pddl_term(&format!("<{}>", subject.trim_matches(|c| c == '<' || c == '>')), notes);
            let p = position_to_pddl_term(&format!("<{}>", predicate.trim_matches(|c| c == '<' || c == '>')), notes);
            let o = position_to_pddl_term(&format!("<{}>", object.trim_matches(|c| c == '<' || c == '>')), notes);
            (false, format!("(triple {} {} {})", s, p, o))
        }
        EffectSpec::AddClass { iri } => {
            let s = position_to_pddl_term(&format!("<{}>", iri.trim_matches(|c| c == '<' || c == '>')), notes);
            let p = sanitise("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
            let o = sanitise("http://www.w3.org/2002/07/owl#Class");
            (true, format!("(triple {} {} {})", s, p, o))
        }
    }
}

/// Emit a PDDL `:action` block for the schema.
fn schema_to_pddl_action(schema: &ActionSchema, notes: &mut Vec<String>) -> String {
    let action_name = sanitise(&schema.name);
    let params: Vec<String> = schema
        .parameters
        .iter()
        .map(|p| format!("?{} - iri", p.name))
        .collect();
    let params_str = if params.is_empty() {
        String::new()
    } else {
        format!(" {}", params.join(" "))
    };

    // Preconditions.
    let mut precond_atoms: Vec<String> = Vec::new();
    for precond in &schema.preconditions {
        if let Some((s, p, o)) = try_extract_ask_triple(precond) {
            let s_t = position_to_pddl_term(&s, notes);
            let p_t = position_to_pddl_term(&p, notes);
            let o_t = position_to_pddl_term(&o, notes);
            precond_atoms.push(format!("(triple {} {} {})", s_t, p_t, o_t));
        } else {
            notes.push(format!(
                "untranslated precondition for action `{}`: {}",
                schema.name, precond
            ));
        }
    }
    let precond_block = if precond_atoms.is_empty() {
        "()".to_string()
    } else if precond_atoms.len() == 1 {
        precond_atoms.remove(0)
    } else {
        format!("(and {})", precond_atoms.join(" "))
    };

    // Effects.
    let mut effect_atoms: Vec<String> = Vec::new();
    for effect in &schema.effects {
        let (positive, atom) = effect_to_pddl(effect, notes);
        if positive {
            effect_atoms.push(atom);
        } else {
            effect_atoms.push(format!("(not {})", atom));
        }
    }
    let effect_block = if effect_atoms.is_empty() {
        "()".to_string()
    } else if effect_atoms.len() == 1 {
        effect_atoms.remove(0)
    } else {
        format!("(and {})", effect_atoms.join(" "))
    };

    format!(
        "  (:action {}\n   :parameters ({})\n   :precondition {}\n   :effect {})",
        action_name,
        params_str.trim(),
        precond_block,
        effect_block
    )
}

/// Compile a PDDL domain from a list of action schemas.
pub fn compile_domain(domain_name: &str, schemas: &[ActionSchema]) -> CompiledPddl {
    let mut notes: Vec<String> = Vec::new();
    let mut actions: Vec<String> = Vec::new();
    for s in schemas {
        actions.push(schema_to_pddl_action(s, &mut notes));
    }

    let domain = format!(
        "(define (domain {})\n  (:requirements :strips :typing :negative-preconditions)\n  (:types iri)\n  (:predicates (triple ?s - iri ?p - iri ?o - iri))\n{}\n)",
        sanitise(domain_name),
        actions.join("\n")
    );

    // Problem stub — empty init/goal; the MCP layer fills in init from the
    // loaded graph and goal from Turtle.
    let problem = String::new();

    CompiledPddl { domain, problem, translation_notes: notes }
}

/// Compile a PDDL problem instance. `init_facts` is the set of `(s, p, o)`
/// IRIs that hold in the start state; `goal_facts` is the set that must hold
/// in the post-state.
pub fn compile_problem(
    problem_name: &str,
    domain_name: &str,
    init_facts: &[(String, String, String)],
    goal_facts: &[(String, String, String)],
) -> String {
    // Collect every distinct IRI to enumerate as objects.
    let mut objs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (s, p, o) in init_facts.iter().chain(goal_facts.iter()) {
        objs.insert(sanitise(s));
        objs.insert(sanitise(p));
        objs.insert(sanitise(o));
    }
    let objs_block: String = objs.into_iter().collect::<Vec<_>>().join(" ");

    let init_block: String = init_facts
        .iter()
        .map(|(s, p, o)| format!("(triple {} {} {})", sanitise(s), sanitise(p), sanitise(o)))
        .collect::<Vec<_>>()
        .join(" ");

    let goal_block: String = if goal_facts.is_empty() {
        "()".to_string()
    } else {
        let atoms: Vec<String> = goal_facts
            .iter()
            .map(|(s, p, o)| format!("(triple {} {} {})", sanitise(s), sanitise(p), sanitise(o)))
            .collect();
        if atoms.len() == 1 {
            atoms[0].clone()
        } else {
            format!("(and {})", atoms.join(" "))
        }
    };

    format!(
        "(define (problem {})\n  (:domain {})\n  (:objects {} - iri)\n  (:init {})\n  (:goal {}))",
        sanitise(problem_name),
        sanitise(domain_name),
        objs_block,
        init_block,
        goal_block
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamics::{ActionSchema, EffectSpec, Parameter};

    fn rename_schema() -> ActionSchema {
        ActionSchema {
            name: "rename_class".to_string(),
            parameters: vec![
                Parameter { name: "old".to_string(), type_iri: None },
                Parameter { name: "new".to_string(), type_iri: None },
            ],
            preconditions: vec![
                "ASK { <{old}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> }".to_string(),
            ],
            effects: vec![
                EffectSpec::AddClass { iri: "{new}".to_string() },
                EffectSpec::RemoveTriple {
                    subject: "{old}".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#Class".to_string(),
                },
            ],
            reversible: false,
            description: None,
            outcomes: vec![],
        }
    }

    #[test]
    fn sanitise_strips_iri_punctuation() {
        let out = sanitise("http://www.w3.org/2002/07/owl#Class");
        assert!(!out.contains('/'));
        assert!(!out.contains(':'));
        assert!(!out.contains('#'));
        assert!(out.contains("class") || out.contains("Class") || out.ends_with("class"));
    }

    #[test]
    fn try_extract_ask_triple_pulls_three_terms() {
        let q = "ASK { <{old}> <http://x.org/p> <http://x.org/q> }";
        let parsed = try_extract_ask_triple(q).expect("parsed");
        assert_eq!(parsed.0, "<{old}>");
        assert_eq!(parsed.1, "<http://x.org/p>");
        assert_eq!(parsed.2, "<http://x.org/q>");
    }

    #[test]
    fn try_extract_ask_returns_none_for_select() {
        let q = "SELECT ?x WHERE { ?x a <http://www.w3.org/2002/07/owl#Class> }";
        assert!(try_extract_ask_triple(q).is_none());
    }

    #[test]
    fn compile_domain_emits_typed_action_with_pddl_keywords() {
        let schemas = vec![rename_schema()];
        let compiled = compile_domain("ontology", &schemas);
        assert!(compiled.domain.contains("(define (domain ontology)"));
        assert!(compiled.domain.contains(":requirements :strips :typing"));
        assert!(compiled.domain.contains("(:predicates (triple"));
        assert!(compiled.domain.contains("(:action rename_class"));
        assert!(compiled.domain.contains("?old - iri"));
        assert!(compiled.domain.contains("?new - iri"));
        assert!(compiled.domain.contains(":precondition (triple ?old"));
        assert!(compiled.domain.contains(":effect (and"));
        assert!(compiled.domain.contains("(not (triple ?old"));
    }

    #[test]
    fn unparseable_precondition_is_recorded_in_notes() {
        let mut s = rename_schema();
        s.preconditions.push("SELECT ?x WHERE { ?x ?p ?o }".to_string());
        let compiled = compile_domain("d", &[s]);
        assert!(
            compiled.translation_notes.iter().any(|n| n.contains("untranslated precondition")),
            "expected a translation note for SELECT-shaped precondition; got: {:?}",
            compiled.translation_notes
        );
    }

    #[test]
    fn compile_problem_renders_objects_init_and_goal() {
        let init = vec![(
            "http://ex.org/Cat".to_string(),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
            "http://www.w3.org/2002/07/owl#Class".to_string(),
        )];
        let goal = vec![(
            "http://ex.org/Feline".to_string(),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
            "http://www.w3.org/2002/07/owl#Class".to_string(),
        )];
        let problem = compile_problem("rename_cat", "ontology", &init, &goal);
        assert!(problem.contains("(:domain ontology)"));
        assert!(problem.contains(":init (triple"));
        assert!(problem.contains(":goal (triple"));
        assert!(problem.contains("- iri"));
    }
}
