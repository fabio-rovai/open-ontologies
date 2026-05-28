//! End-to-end runnable demo of the three-layer architecture
//! (Dynamics #43 → Causal hookup → Planner #45) shipped on PR #46.
//!
//! Run with: `cargo run --example three_layer_pipeline`
//!
//! What it demonstrates, top to bottom:
//!
//!   1. Register two action schemas:
//!      - `add_subclass_edge` (deterministic)
//!      - `noisy_add_class`  (non-deterministic, #49)
//!   2. Compile PDDL from those schemas + a goal Turtle slice.
//!   3. Simulate what a Fast Downward run would have produced
//!      (no Fast Downward binary required — we use a hardcoded sas_plan
//!      that the parser eats).
//!   4. Parse the sas_plan, bind PDDL args back to original IRIs.
//!   5. Validate the plan against the graph in a sandbox.
//!   6. For each step: CIVeX-certify it, then apply with ramification.
//!   7. Print the resulting graph state.
//!
//! No external dependencies (Python / DoWhy / Fast Downward) required —
//! every layer is exercised through its in-process API.

use open_ontologies::civex::{certify_action, ActionFrame, Verdict};
use open_ontologies::dynamics::{
    list_names, lookup, register, ActionSchema, EffectSpec, Outcome, Parameter,
};
use open_ontologies::graph::GraphStore;
use open_ontologies::plan_classical::parse_sas_plan;
use open_ontologies::plan_pddl::{compile_domain, compile_problem};
use open_ontologies::plan_validate::{validate_plan, PlanStep};
use open_ontologies::state::StateDb;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

fn print_section(title: &str) {
    println!("\n{}\n{}\n", title, "─".repeat(title.len()));
}

fn build_add_subclass_schema() -> ActionSchema {
    ActionSchema {
        name: "add_subclass_edge".to_string(),
        parameters: vec![
            Parameter { name: "child".to_string(), type_iri: None },
            Parameter { name: "parent".to_string(), type_iri: None },
        ],
        preconditions: vec![
            "ASK { <{child}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> }".to_string(),
            "ASK { <{parent}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> }".to_string(),
        ],
        effects: vec![EffectSpec::AddTriple {
            subject: "{child}".to_string(),
            predicate: "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
            object: "{parent}".to_string(),
        }],
        reversible: true,
        description: Some("Add a subClassOf edge between two existing classes".into()),
        outcomes: vec![],
    }
}

fn build_noisy_add_class_schema() -> ActionSchema {
    // 80% chance: declare the IRI as an owl:Class normally.
    // 20% chance: also tag it with an audit-marker class. Models a noisy
    // LLM proposer that sometimes over-asserts. The CIVeX cert + ramification
    // are robust to both outcomes.
    ActionSchema {
        name: "noisy_add_class".to_string(),
        parameters: vec![Parameter { name: "iri".to_string(), type_iri: None }],
        preconditions: vec![],
        effects: vec![], // ignored when outcomes is non-empty
        reversible: true,
        description: Some("Add an owl:Class with a noisy LLM-style proposer".into()),
        outcomes: vec![
            Outcome {
                probability: 0.8,
                effects: vec![EffectSpec::AddClass { iri: "{iri}".to_string() }],
                label: Some("clean".to_string()),
            },
            Outcome {
                probability: 0.2,
                effects: vec![
                    EffectSpec::AddClass { iri: "{iri}".to_string() },
                    EffectSpec::AddTriple {
                        subject: "{iri}".to_string(),
                        predicate: "http://example.org/audit#flaggedBy".to_string(),
                        object: "http://example.org/audit#NoisyProposer".to_string(),
                    },
                ],
                label: Some("audit_flagged".to_string()),
            },
        ],
    }
}

fn main() -> anyhow::Result<()> {
    // ── Setup ───────────────────────────────────────────────────────────
    let db = StateDb::open(Path::new(":memory:"))?;
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(
        r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex:  <http://ex.org/> .
        ex:Animal a owl:Class .
        ex:Cat a owl:Class .
        ex:tigger a ex:Cat .
    "#,
        None,
    )?;

    print_section("0. Seed graph");
    println!("Loaded {} triples", graph.triple_count());

    // ── Layer 1: Dynamics — register schemas ────────────────────────────
    print_section("1. Dynamics: register two schemas");
    register(&db, &build_add_subclass_schema())?;
    register(&db, &build_noisy_add_class_schema())?;
    println!("Registered: {:?}", list_names(&db)?);

    // ── Layer 3: Planner — compile PDDL ─────────────────────────────────
    print_section("2. Planner: compile PDDL domain from registered schemas");
    let schemas: Vec<ActionSchema> = list_names(&db)?
        .iter()
        .filter_map(|n| lookup(&db, n).ok().flatten())
        .collect();
    let compiled = compile_domain("ontology", &schemas);
    println!("PDDL domain (first 12 lines):");
    for line in compiled.domain.lines().take(12) {
        println!("  {}", line);
    }
    if !compiled.translation_notes.is_empty() {
        println!("Translation notes (lossy):");
        for n in &compiled.translation_notes {
            println!("  - {}", n);
        }
    }

    // Compile a problem instance from the seed graph + a goal.
    let init_facts: Vec<(String, String, String)> = graph
        .all_triples()?
        .into_iter()
        .map(|(s, p, o)| {
            (
                s.trim_matches(|c| c == '<' || c == '>').to_string(),
                p.trim_matches(|c| c == '<' || c == '>').to_string(),
                o.trim_matches(|c| c == '<' || c == '>').to_string(),
            )
        })
        .collect();
    let goal = vec![(
        "http://ex.org/Cat".to_string(),
        "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
        "http://ex.org/Animal".to_string(),
    )];
    let problem = compile_problem("demo_problem", "ontology", &init_facts, &goal);
    println!("\nPDDL problem (first 4 lines):");
    for line in problem.lines().take(4) {
        println!("  {}", line);
    }

    // ── Layer 3: Planner — simulate the solver's output ─────────────────
    print_section("3. Planner: parse sas_plan from a (mock) Fast Downward run");
    // What Fast Downward would emit, given the goal:
    //   1. Declare a new class (noisy_add_class chooses one of two outcomes).
    //   2. Connect Cat → Animal via subClassOf.
    let mock_sas_plan = "\
(noisy_add_class http___ex_org_mammal)\n\
(add_subclass_edge http___ex_org_cat http___ex_org_animal)\n\
; cost = 2 (unit cost)\n";
    let (operators, cost) = parse_sas_plan(mock_sas_plan);
    println!("Parsed {} operators (cost: {:?})", operators.len(), cost);
    for (i, op) in operators.iter().enumerate() {
        println!("  step {}: {} {:?}", i, op.name, op.args);
    }

    // ── Layer 3: Planner — bind PDDL args back to original IRIs ─────────
    print_section("4. Bind PDDL args back to IRIs (orchestrator side)");
    // The orchestrator knows the schemas it registered, so it can map each
    // operator's positional args to the schema's parameter names. PDDL
    // identifiers are sanitised IRIs (`/` `:` `#` → `_`); for this demo we
    // hardcode the inverse mapping the orchestrator would maintain.
    let iri_map: BTreeMap<&str, &str> = BTreeMap::from([
        ("http___ex_org_mammal", "http://ex.org/Mammal"),
        ("http___ex_org_cat", "http://ex.org/Cat"),
        ("http___ex_org_animal", "http://ex.org/Animal"),
    ]);
    let mut plan_steps: Vec<PlanStep> = Vec::new();
    for op in &operators {
        let schema = lookup(&db, &op.name)?
            .ok_or_else(|| anyhow::anyhow!("unknown action `{}` in plan", op.name))?;
        let mut bindings = BTreeMap::new();
        for (param, arg) in schema.parameters.iter().zip(op.args.iter()) {
            let resolved = iri_map.get(arg.as_str()).copied().unwrap_or(arg.as_str()).to_string();
            bindings.insert(param.name.clone(), resolved);
        }
        println!("  {} bindings: {:?}", op.name, bindings);
        plan_steps.push(PlanStep { action_name: op.name.clone(), bindings });
    }

    // ── Layer 3: Planner — validate the bound plan in a sandbox ─────────
    print_section("5. Validate the bound plan (no mutation of the real graph)");
    let goal_facts_nt: Vec<(String, String, String)> = goal
        .iter()
        .map(|(s, p, o)| (format!("<{}>", s), format!("<{}>", p), format!("<{}>", o)))
        .collect();
    let report = validate_plan(&db, &graph, &plan_steps, &goal_facts_nt)?;
    println!(
        "valid={} steps={}/{} initial_triples={} final_triples={} unsatisfied_goals={}",
        report.valid,
        report.steps_validated,
        report.steps_total,
        report.initial_triple_count,
        report.final_triple_count,
        report.unsatisfied_goals.len()
    );
    // The real graph must still be untouched at this point.
    assert_eq!(graph.triple_count(), 3, "sandbox must not mutate real graph");

    // ── Layer 2: Causal — certify + apply each step ─────────────────────
    print_section("6. Per-step: CIVeX certify, then apply with ramification");
    for (i, step) in plan_steps.iter().enumerate() {
        let schema = lookup(&db, &step.action_name)?.unwrap();
        let bindings: Vec<(String, String)> = step
            .bindings
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Build a CIVeX ActionFrame describing the change.
        let proposed_ttl = synthesise_proposed_ttl(&schema, &bindings);
        let target_iris: Vec<String> = schema
            .parameters
            .first()
            .and_then(|p| bindings.iter().find(|(k, _)| k == &p.name))
            .map(|(_, v)| vec![v.clone()])
            .unwrap_or_default();

        let frame = ActionFrame {
            tool: "onto_action_apply".to_string(),
            target_iris,
            proposed_delta_ttl: proposed_ttl,
            utility_metric: "dependent_query_pass_rate".to_string(),
            dependent_queries: vec![
                "SELECT ?x WHERE { ?x a <http://www.w3.org/2002/07/owl#Class> }".to_string(),
                "SELECT ?x WHERE { ?x ?p ?o } LIMIT 5".to_string(),
                "SELECT ?p WHERE { ?s ?p ?o } LIMIT 5".to_string(),
                "SELECT ?o WHERE { ?s ?p ?o } LIMIT 5".to_string(),
                "SELECT ?s WHERE { ?s ?p ?o } LIMIT 5".to_string(),
            ],
            cost_threshold: 1000,
            utility_threshold: 0.5,
            risk_threshold: 5000,
            reversible: schema.reversible,
            allow_experiment: false,
            alpha: 0.05,
            action_schema_name: Some(schema.name.clone()),
        };
        let cert = certify_action(&db, &graph, &frame)?;
        println!(
            "  step {} `{}`: verdict={:?}, utility_lcb={:.3}",
            i, step.action_name, cert.certificate.verdict, cert.certificate.utility_lcb
        );

        if !matches!(cert.certificate.verdict, Verdict::Execute) {
            println!("    → skipping (verdict not EXECUTE)");
            continue;
        }

        // Apply with OWL-RL ramification + a deterministic seed so the
        // non-deterministic outcome is reproducible across runs of the demo.
        let result =
            schema.apply_with_ramification(&graph, &db, &bindings, "owl-rl")?;
        // For nondeterministic schemas, apply_with_ramification used SystemTime
        // seeding inside apply(). For a deterministic demo we'd use
        // apply_with_seed first; for clarity here we just print what landed.
        println!(
            "    applied: +{} triples, removed {}, derived +{} via OWL-RL, outcome={:?}",
            result.triples_added,
            result.triples_removed,
            result.derived_triples_added,
            result.sampled_outcome_label
        );
    }

    // ── Final state ─────────────────────────────────────────────────────
    print_section("7. Final graph state");
    println!("Total triples: {}", graph.triple_count());
    let ask = graph.sparql_select(
        "ASK { <http://ex.org/Cat> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://ex.org/Animal> }",
    )?;
    println!("Cat rdfs:subClassOf Animal? {}", if ask.contains("\"result\":true") { "YES" } else { "no" });
    let ask2 = graph.sparql_select(
        "ASK { <http://ex.org/tigger> a <http://ex.org/Animal> }",
    )?;
    println!(
        "tigger a Animal (entailed via OWL-RL ramification)? {}",
        if ask2.contains("\"result\":true") { "YES" } else { "no" }
    );

    println!("\nDone. Every layer of the three-layer architecture exercised through its public API.");
    Ok(())
}

/// Synthesise the Turtle delta that the schema would produce under the
/// supplied bindings — used as `proposed_delta_ttl` for the CIVeX frame.
fn synthesise_proposed_ttl(schema: &ActionSchema, bindings: &[(String, String)]) -> String {
    // For non-deterministic schemas we project the "headline" effects from
    // the most-probable outcome; CIVeX is judging the structural change, not
    // the precise stochastic branch. For deterministic schemas we use the
    // literal effects.
    let effects: &[EffectSpec] = if schema.outcomes.is_empty() {
        &schema.effects
    } else {
        schema
            .outcomes
            .iter()
            .max_by(|a, b| {
                a.probability
                    .partial_cmp(&b.probability)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|o| o.effects.as_slice())
            .unwrap_or(&[])
    };
    let mut ttl = String::new();
    for effect in effects {
        match effect {
            EffectSpec::AddTriple { subject, predicate, object } => {
                let s = schema.substitute(subject, bindings);
                let p = schema.substitute(predicate, bindings);
                let o = schema.substitute(object, bindings);
                ttl.push_str(&format!("<{}> <{}> <{}> .\n", s, p, o));
            }
            EffectSpec::RemoveTriple { subject, predicate, object } => {
                // Removal isn't expressible in Turtle directly; CIVeX uses the
                // delta as the post-state under structural-dependency analysis,
                // so we omit removals here and rely on the lock-IRI machinery
                // for risk accounting.
                let _ = (subject, predicate, object);
            }
            EffectSpec::AddClass { iri } => {
                let s = schema.substitute(iri, bindings);
                ttl.push_str(&format!(
                    "<{}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> .\n",
                    s
                ));
            }
        }
    }
    ttl
}
