//! **Issue #192.** A false clean: `$this` inside a sub-SELECT.
//!
//! `prebinding_violation` already refused `MINUS`, `SERVICE`, a `VALUES` over
//! the pre-bound variable, and a sub-SELECT that does not project it. But it
//! EXEMPTED `SELECT *`, and that exemption is wrong.
//!
//! A subquery's `*` projects the variables that subquery BINDS. The pre-bound
//! variable is bound outside it, so the substitution does not reach inward and
//! the inner reference is simply unbound. The query then finds no solution,
//! and a SPARQL constraint with no solution is a constraint that HELD.
//!
//! So the run reported `conforms: true` with an empty `skipped_constraints`,
//! about a constraint it had never evaluated. That is a false clean, and the
//! quiet kind: nothing in the report said anything had gone unchecked.
//!
//! The W3C suite settles the question rather than this file deciding it.
//! `sparql/pre-binding/pre-binding-006` is exactly this query shape and its
//! expected result is `sht:Failure`, a refusal rather than a verdict.
//! pySHACL 0.40.1 raises `ValidationFailure` on it, which is the same answer.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

const DATA: &str = r#"
    @prefix ex: <http://example.org/> .
    ex:InvalidResource a ex:Thing .
    ex:ValidResource   a ex:Thing .
"#;

/// `$this` reached only from inside a nested SELECT that projects `*`.
const SUBQUERY: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:SubqueryShape a sh:NodeShape ;
        sh:targetNode ex:InvalidResource , ex:ValidResource ;
        sh:sparql [
            sh:select """
                SELECT $this
                WHERE {
                    { SELECT * WHERE { FILTER ($this = <http://example.org/InvalidResource>) } }
                }"""
        ] .
"#;

/// The same constraint with the FILTER at the top level. This is W3C
/// `pre-binding-001`, whose expected result is one violation on
/// `ex:InvalidResource`, and it is the control: without it, a fix that refused
/// every SPARQL constraint would pass the test above and look correct.
const PLAIN: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:PlainShape a sh:NodeShape ;
        sh:targetNode ex:InvalidResource , ex:ValidResource ;
        sh:sparql [
            sh:select """
                SELECT $this
                WHERE { FILTER ($this = <http://example.org/InvalidResource>) }"""
        ] .
"#;

fn report(shapes: &str) -> serde_json::Value {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(DATA, None).expect("data must parse");
    let json = ShaclValidator::validate(&graph, shapes).expect("validate");
    serde_json::from_str(&json).expect("report is JSON")
}

#[test]
fn a_subquery_that_cannot_receive_the_binding_is_refused_and_not_called_conforming() {
    let r = report(SUBQUERY);
    assert!(
        r["conforms"].is_null(),
        "the validator cannot evaluate this constraint, so it must withhold the verdict. \
         Reporting true states that a constraint held when it was never checked, which is the \
         one failure mode a validator used as an assurance gate must not have: {r}"
    );
    let skipped = r["skipped_constraints"].as_array().expect("skipped_constraints");
    // One entry per constraint-evaluation SITE, so a shape with two target
    // nodes produces two. That granularity is documented on `onto_shacl`.
    assert_eq!(
        skipped.len(),
        2,
        "the refusal must be VISIBLE, once per evaluation site. A null verdict with nothing in \
         skipped_constraints leaves a reader unable to tell a refusal from a clean run: {r}"
    );
    for entry in skipped {
        assert!(
            entry["reason"].as_str().is_some_and(|s| s.contains("sub-SELECT")),
            "the reason must name the construct, so the author can fix the query: {entry}"
        );
    }
}

#[test]
fn the_same_constraint_at_the_top_level_is_still_evaluated() {
    let r = report(PLAIN);
    assert_eq!(
        r["conforms"],
        serde_json::json!(false),
        "pre-binding works at the top level and must keep working; a fix that refused every \
         SPARQL constraint would satisfy the test above and be worthless: {r}"
    );
    assert_eq!(r["violation_count"], serde_json::json!(1), "exactly ex:InvalidResource: {r}");
    assert_eq!(
        r["violations"][0]["focus_node"],
        serde_json::json!("http://example.org/InvalidResource"),
        "and the violation is on the node the FILTER selects: {r}"
    );
    // The key is omitted entirely when nothing was skipped, so absent and
    // empty both mean the same thing and either is acceptable.
    let skipped = r["skipped_constraints"].as_array().map(|s| s.len()).unwrap_or(0);
    assert_eq!(skipped, 0, "nothing should be skipped here: {r}");
}
