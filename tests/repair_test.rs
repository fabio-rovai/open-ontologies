//! Weighted repair, on fixtures whose optimum is worked out by hand.
//!
//! Each fixture is small enough that the cheapest repair can be checked on
//! paper, which is the only way to test an optimiser: a test that asserts
//! whatever the search returned tests nothing. The divergence test is the one
//! that matters most. Summing raw confidences and summing log-odds pick
//! DIFFERENT repairs on an ordinary input, and a tool that silently used the
//! first while calling it the most probable repair would be wrong in a way no
//! reader could see.

use open_ontologies::defects::Defects;
use open_ontologies::graph::GraphStore;
use open_ontologies::repair::{RepairOptions, WeightKind, render_human, repair};
use serde_json::Value;
use std::collections::BTreeSet;
use std::process::Command;
use std::sync::Arc;

const W: &str = "https://example.org/weight#confidence";
const SUB: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const DISJ: &str = "http://www.w3.org/2002/07/owl#disjointWith";

const PREFIXES: &str = r#"
@prefix ex: <https://example.org/x/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix w: <https://example.org/weight#> .
"#;

/// The example from issue #346, weights as reified annotations.
const ISSUE: &str = r#"
ex:A a owl:Class . ex:B a owl:Class . ex:C a owl:Class . ex:D a owl:Class .
ex:A rdfs:subClassOf ex:B .
ex:A owl:disjointWith ex:B .
ex:C rdfs:subClassOf ex:A .
ex:D rdfs:subClassOf ex:A .
[] a owl:Axiom ; owl:annotatedSource ex:A ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:B ; w:confidence "0.87"^^xsd:decimal .
[] a owl:Axiom ; owl:annotatedSource ex:A ; owl:annotatedProperty owl:disjointWith ;
   owl:annotatedTarget ex:B ; w:confidence "0.62"^^xsd:decimal .
[] a owl:Axiom ; owl:annotatedSource ex:C ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:A ; w:confidence "0.99"^^xsd:decimal .
[] a owl:Axiom ; owl:annotatedSource ex:D ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:A ; w:confidence "0.99"^^xsd:decimal .
"#;

/// Two classes inherit one disjointness. With the unannotated edges out of
/// play, the soft parts of the two conflicts are `{a=0.6, c=0.9}` and
/// `{b=0.6, c=0.9}`: `a` is `P subClassOf X`, `b` is `Q subClassOf X`, `c` is
/// `X disjointWith Y`.
const DIVERGE: &str = r#"
ex:P rdfs:subClassOf ex:X , ex:Y .
ex:Q rdfs:subClassOf ex:X , ex:Y .
ex:X owl:disjointWith ex:Y .
[] a owl:Axiom ; owl:annotatedSource ex:P ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:X ; w:confidence 0.6 .
[] a owl:Axiom ; owl:annotatedSource ex:Q ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:X ; w:confidence 0.6 .
[] a owl:Axiom ; owl:annotatedSource ex:X ; owl:annotatedProperty owl:disjointWith ;
   owl:annotatedTarget ex:Y ; w:confidence 0.9 .
"#;

fn store(body: &str) -> Arc<GraphStore> {
    let g = GraphStore::new();
    g.load_turtle(&format!("{PREFIXES}{body}"), None).expect("the fixture parses");
    Arc::new(g)
}

fn opts(weights: WeightKind, default_weight: f64) -> RepairOptions {
    RepairOptions {
        weight_property: Some(W.to_string()),
        default_weight,
        weights,
        ..Default::default()
    }
}

fn ex(local: &str) -> String {
    format!("https://example.org/x/{local}")
}

/// The dropped axioms as `"S P O"` with local names, sorted.
fn dropped(v: &Value) -> BTreeSet<String> {
    v["dropped"]
        .as_array()
        .expect("dropped is an array")
        .iter()
        .map(|d| {
            let a = &d["axiom"];
            let local = |k: &str| {
                let s = a[k].as_str().unwrap();
                s.rsplit(['/', '#']).next().unwrap().to_string()
            };
            format!("{} {} {}", local("s"), local("p"), local("o"))
        })
        .collect()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn classes(v: &Value, key: &str) -> BTreeSet<String> {
    v[key]["classes"]
        .as_array()
        .unwrap_or_else(|| panic!("{key} lists classes: {v}"))
        .iter()
        .map(|c| c.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn the_issue_example_drops_the_disjointness_and_makes_three_classes_coherent() {
    let v = repair(&store(ISSUE), &opts(WeightKind::Cost, 1.0)).unwrap();
    assert_eq!(dropped(&v), set(&["A disjointWith B"]), "{v:#}");
    assert_eq!(v["total_weight"], 0.62, "{v:#}");
    assert_eq!(v["optimal"], true);
    assert_eq!(classes(&v, "incoherent_before"), [ex("A"), ex("C"), ex("D")].into_iter().collect());
    assert_eq!(classes(&v, "classes_made_coherent"), [ex("A"), ex("C"), ex("D")].into_iter().collect());
    assert_eq!(v["coherence_after"]["verdict"], "coherent_in_this_fragment");

    // The one conflict is the issue's MinA, re-run both ways.
    let c = &v["conflicts"][0];
    assert_eq!(v["conflict_count"], 1, "C and D are derived from A, so only A is explained: {v:#}");
    assert_eq!(c["class"], ex("A"));
    assert_eq!(c["size"], 2);
    assert_eq!(c["sufficiency"], "re_run_and_incoherent");
    assert_eq!(c["minimality"], "re_run_without_each_axiom_and_coherent");
    assert_eq!(v["dropped"][0]["hits"], serde_json::json!([0]));
    assert_eq!(v["dropped"][0]["weight_source"], "annotation");
    assert_eq!(v["conflicts_verified"], true);

    // Decimal weights with two places are scaled by 100, exactly.
    assert_eq!(v["cost_scale"], 100);
    assert_eq!(v["cost_scaling_exact"], true);
    assert_eq!(v["total_cost_scaled"], 62);
    assert_eq!(v["certificate"]["status"], "not_requested");

    let text = render_human(&v);
    assert!(text.contains("drop 1 axiom"), "{text}");
    assert!(text.contains("owl:disjointWith"), "{text}");
}

#[test]
fn cost_and_probability_weights_choose_different_repairs() {
    // Under `cost` the unannotated edges cost 100 each, so they stay out of
    // play: dropping c costs 0.9 and dropping a and b costs 1.2.
    let cost = repair(&store(DIVERGE), &opts(WeightKind::Cost, 100.0)).unwrap();
    assert_eq!(dropped(&cost), set(&["X disjointWith Y"]), "{cost:#}");
    assert!((cost["total_weight"].as_f64().unwrap() - 0.9).abs() < 1e-12, "{cost:#}");
    assert_eq!(cost["optimal"], true);

    // Under `probability` the unannotated edges are p = 1, so hard. The
    // objective is the log-odds of what is dropped: 2 x ln(0.6/0.4) = 0.811
    // beats ln(0.9/0.1) = 2.197.
    let prob = repair(&store(DIVERGE), &opts(WeightKind::Probability, 1.0)).unwrap();
    assert_eq!(dropped(&prob), set(&["P subClassOf X", "Q subClassOf X"]), "{prob:#}");
    let expected = 2.0 * (0.6f64 / 0.4).ln();
    assert!((prob["total_weight"].as_f64().unwrap() - expected).abs() < 1e-9, "{prob:#}");
    assert_eq!(prob["optimal"], true);
    assert_eq!(prob["weights"], "probability");
    assert_eq!(prob["cost_scale"], 1_000_000);
    for d in prob["dropped"].as_array().unwrap() {
        assert_eq!(d["weight"], 0.6);
        assert!((d["cost"].as_f64().unwrap() - (0.6f64 / 0.4).ln()).abs() < 1e-12);
        assert_eq!(d["reason"], "minimum_cost_hitting_set");
    }
    // Same graph, same conflicts, different objective.
    assert_eq!(cost["conflict_count"], 2);
    assert_eq!(prob["conflict_count"], 2);
}

#[test]
fn a_conflict_of_hard_axioms_only_is_unrepairable_and_said_so() {
    // H sits under X and Y, X disjoint with Y, all unannotated, so all p = 1.
    // S sits under H by a SOFT edge and can be rescued by dropping that edge.
    // K is an ordinary repair: dropping K subClassOf Z (ln(0.7/0.3) = 0.847)
    // beats dropping Z disjointWith W (ln(0.8/0.2) = 1.386).
    let body = r#"
ex:H rdfs:subClassOf ex:X , ex:Y .
ex:X owl:disjointWith ex:Y .
ex:S rdfs:subClassOf ex:H .
ex:K rdfs:subClassOf ex:Z , ex:W .
ex:Z owl:disjointWith ex:W .
[] a owl:Axiom ; owl:annotatedSource ex:S ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:H ; w:confidence 0.9 .
[] a owl:Axiom ; owl:annotatedSource ex:K ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:Z ; w:confidence 0.7 .
[] a owl:Axiom ; owl:annotatedSource ex:Z ; owl:annotatedProperty owl:disjointWith ;
   owl:annotatedTarget ex:W ; w:confidence 0.8 .
"#;
    let v = repair(&store(body), &opts(WeightKind::Probability, 1.0)).unwrap();
    assert_eq!(v["unrepairable"]["count"], 1, "{v:#}");
    let u = &v["unrepairable"]["conflicts"][0];
    assert_eq!(u["class"], ex("H"));
    assert_eq!(u["axioms"].as_array().unwrap().len(), 3, "{u}");
    assert_eq!(dropped(&v), set(&["K subClassOf Z", "S subClassOf H"]), "{v:#}");
    assert_eq!(v["optimal"], true, "optimal within the soft axioms: {v:#}");
    assert_eq!(classes(&v, "still_incoherent"), [ex("H")].into_iter().collect());
    assert!(classes(&v, "classes_made_coherent").contains(&ex("S")));
    assert_eq!(
        v["coherence_after"]["verdict"],
        "coherent_in_this_fragment_except_the_unrepairable_classes"
    );
    // No hard axiom is ever dropped.
    for d in v["dropped"].as_array().unwrap() {
        assert_ne!(d["weight"], 1.0, "{d}");
    }
}

#[test]
fn a_probability_at_most_one_half_is_dropped_at_no_cost_and_said_so() {
    // R subClassOf T at p = 0.4 is in a conflict: dropping it raises the
    // probability, so it goes at no cost. V subClassOf T at p = 0.3 is in no
    // conflict and is kept, and the count of such axioms is reported.
    let body = r#"
ex:R rdfs:subClassOf ex:T , ex:U .
ex:T owl:disjointWith ex:U .
ex:V rdfs:subClassOf ex:T .
[] a owl:Axiom ; owl:annotatedSource ex:R ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:T ; w:confidence 0.4 .
[] a owl:Axiom ; owl:annotatedSource ex:T ; owl:annotatedProperty owl:disjointWith ;
   owl:annotatedTarget ex:U ; w:confidence 0.95 .
[] a owl:Axiom ; owl:annotatedSource ex:V ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:T ; w:confidence 0.3 .
"#;
    let v = repair(&store(body), &opts(WeightKind::Probability, 1.0)).unwrap();
    assert_eq!(dropped(&v), set(&["R subClassOf T"]), "{v:#}");
    let d = &v["dropped"][0];
    assert_eq!(d["reason"], "probability_at_most_one_half_dropped_at_no_cost");
    assert_eq!(d["cost"], 0.0);
    assert_eq!(v["total_weight"], 0.0);
    assert_eq!(v["dropped_at_no_cost"], 1);
    assert_eq!(v["low_probability_axioms_kept"]["count"], 1, "{v:#}");
    assert_eq!(v["optimal"], true);
}

#[test]
fn two_independent_paths_need_a_second_round() {
    // C reaches X through P1 and through P2, and reaches Y directly. The first
    // conflict found goes through one path; its cheapest axiom is the edge
    // into that path, and dropping it leaves C incoherent through the other.
    // The second round collects that conflict and the optimum drops both
    // cheap edges (0.2) rather than C subClassOf Y or X disjointWith Y (0.9).
    let body = r#"
ex:C rdfs:subClassOf ex:P1 , ex:P2 , ex:Y .
ex:P1 rdfs:subClassOf ex:X .
ex:P2 rdfs:subClassOf ex:X .
ex:X owl:disjointWith ex:Y .
[] a owl:Axiom ; owl:annotatedSource ex:C ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:P1 ; w:confidence 0.1 .
[] a owl:Axiom ; owl:annotatedSource ex:C ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:P2 ; w:confidence 0.1 .
"#;
    let v = repair(&store(body), &opts(WeightKind::Cost, 0.9)).unwrap();
    assert_eq!(dropped(&v), set(&["C subClassOf P1", "C subClassOf P2"]), "{v:#}");
    assert!(v["optimality"]["rounds"].as_u64().unwrap() >= 2, "{v:#}");
    assert_eq!(v["conflict_count"], 2);
    assert_eq!(v["conflicts"][1]["found_in_round"], 2);
    assert_eq!(v["optimal"], true);
    assert!((v["total_weight"].as_f64().unwrap() - 0.2).abs() < 1e-12);
    assert_eq!(v["lower_bound_scaled"], v["total_cost_scaled"]);
    // Each dropped edge breaks exactly one of the two conflicts.
    for d in v["dropped"].as_array().unwrap() {
        assert_eq!(d["hits"].as_array().unwrap().len(), 1, "{d}");
    }
}

#[test]
fn an_already_coherent_ontology_gets_the_empty_repair() {
    let body = r#"
ex:Dog rdfs:subClassOf ex:Animal .
ex:Cat rdfs:subClassOf ex:Animal .
ex:Dog owl:disjointWith ex:Cat .
"#;
    let v = repair(&store(body), &opts(WeightKind::Cost, 1.0)).unwrap();
    assert_eq!(v["dropped_count"], 0);
    assert_eq!(v["optimal"], true);
    assert_eq!(v["conflict_count"], 0);
    assert_eq!(v["optimality"]["rounds"], 0);
    assert_eq!(v["incoherent_before"]["count"], 0);
    assert_eq!(v["lower_bound"], 0.0);
    assert_eq!(v["coherence_after"]["verdict"], "coherent_in_this_fragment");
}

#[test]
fn a_class_disjoint_with_itself_is_incoherent_here() {
    // The one judgement beyond `defects`, which does not look for it.
    let v = repair(&store("ex:E owl:disjointWith ex:E .\nex:F rdfs:subClassOf ex:E ."), &opts(WeightKind::Cost, 1.0))
        .unwrap();
    assert_eq!(classes(&v, "incoherent_before"), [ex("E"), ex("F")].into_iter().collect());
    assert_eq!(dropped(&v), set(&["E disjointWith E"]), "{v:#}");
}

#[test]
fn equivalent_class_is_one_axiom_and_two_edges() {
    // G equivalentClass H with H subClassOf both sides of a disjointness: one
    // axiom, so one thing to drop, even though it contributes two edges.
    let body = r#"
ex:G owl:equivalentClass ex:H .
ex:G rdfs:subClassOf ex:L .
ex:H rdfs:subClassOf ex:M .
ex:L owl:disjointWith ex:M .
[] a owl:Axiom ; owl:annotatedSource ex:G ; owl:annotatedProperty owl:equivalentClass ;
   owl:annotatedTarget ex:H ; w:confidence 0.5 .
"#;
    let v = repair(&store(body), &opts(WeightKind::Cost, 2.0)).unwrap();
    assert_eq!(dropped(&v), set(&["G equivalentClass H"]), "{v:#}");
    assert_eq!(v["scope"]["axioms_considered"]["equivalentClass"], 1);
    assert_eq!(v["coherence_after"]["verdict"], "coherent_in_this_fragment");
}

#[test]
fn axioms_outside_the_fragment_are_counted_and_never_read() {
    let body = r#"
ex:A rdfs:subClassOf ex:B .
ex:A owl:disjointWith ex:B .
ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom ex:B ] .
[] a owl:AllDisjointClasses ; owl:members ( ex:B ex:C ) .
ex:p rdfs:domain ex:A .
ex:A rdfs:label "A" .
"#;
    let v = repair(&store(body), &opts(WeightKind::Cost, 1.0)).unwrap();
    let scope = &v["scope"];
    assert_eq!(scope["coherence_judged_in_this_fragment_only"], true);
    assert_eq!(scope["axioms_considered_total"], 2);
    assert_eq!(scope["not_considered"]["subClassOf_not_between_two_named_classes"], 1, "{scope:#}");
    assert_eq!(scope["not_considered"]["AllDisjointClasses"], 1, "{scope:#}");
    assert_eq!(scope["not_considered"]["property_axioms"], 1, "{scope:#}");
    assert_eq!(scope["not_considered_total"], 3);
    // Every triple lands in one bucket and the buckets add up.
    let g = store(body);
    let total: u64 = scope["triples"].as_object().unwrap().values().map(|n| n.as_u64().unwrap()).sum();
    assert_eq!(total, g.triple_count() as u64, "{scope:#}");
    assert_eq!(scope["triples_total"], total);
    // Without a weight property, every axiom carries the default.
    let plain = repair(&g, &RepairOptions::default()).unwrap();
    assert_eq!(plain["dropped"][0]["weight_source"], "default");
    assert_eq!(plain["weight_annotations_used"], 0);
}

#[test]
fn a_weight_that_cannot_be_read_refuses_the_repair() {
    let body = r#"
ex:A rdfs:subClassOf ex:B .
ex:A owl:disjointWith ex:B .
[] a owl:Axiom ; owl:annotatedSource ex:A ; owl:annotatedProperty owl:disjointWith ;
   owl:annotatedTarget ex:B ; w:confidence "high" .
"#;
    let err = repair(&store(body), &opts(WeightKind::Cost, 1.0)).unwrap_err().to_string();
    assert!(err.contains("not a number"), "{err}");
    let bad = repair(&store(ISSUE), &opts(WeightKind::Probability, 1.5)).unwrap_err().to_string();
    assert!(bad.contains("not a probability"), "{bad}");
}

#[test]
fn a_bound_that_fires_is_named_and_the_answer_is_not_called_optimal() {
    // P reaches X, Y and Z by three soft edges at p = 0.9, and the three are
    // pairwise disjoint by hard axioms. The conflicts' soft parts form a
    // triangle, {a,b}, {b,c}, {a,c}: the optimum drops two edges, and neither
    // root bound reaches that (packing finds one edge, apportioning 1.5), so
    // one node cannot close the search.
    let body = r#"
ex:P rdfs:subClassOf ex:X , ex:Y , ex:Z .
ex:X owl:disjointWith ex:Y . ex:Y owl:disjointWith ex:Z . ex:X owl:disjointWith ex:Z .
[] a owl:Axiom ; owl:annotatedSource ex:P ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:X ; w:confidence 0.9 .
[] a owl:Axiom ; owl:annotatedSource ex:P ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:Y ; w:confidence 0.9 .
[] a owl:Axiom ; owl:annotatedSource ex:P ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:Z ; w:confidence 0.9 .
"#;
    let mut o = opts(WeightKind::Probability, 1.0);
    o.max_nodes = 1;
    o.certificate_dir = Some(std::env::temp_dir().join("oo-repair-never-written"));
    let v = repair(&store(body), &o).unwrap();
    assert_eq!(v["optimal"], false, "{v:#}");
    assert_eq!(v["optimality"]["bound_fired"], "max_nodes");
    assert_eq!(v["dropped_count"], 2, "the best found is still the optimum here: {v:#}");
    let lb = v["lower_bound"].as_f64().unwrap();
    let total = v["total_weight"].as_f64().unwrap();
    assert!(lb > 0.0 && lb < total, "a proved bound below the repair: {lb} vs {total}");
    assert_eq!(v["coherence_after"]["verdict"], "coherent_in_this_fragment", "still a repair");
    assert_eq!(v["certificate"]["status"], "not_attempted");
    assert_eq!(v["dropped"][0]["reason"], "best_repair_found_not_proved_minimum");
    // With room the same instance closes, and is optimal.
    o.max_nodes = 1_000;
    o.certificate_dir = None;
    let v = repair(&store(body), &o).unwrap();
    assert_eq!(v["optimal"], true, "{v:#}");
    assert_eq!(v["dropped_count"], 2);

    let mut r = opts(WeightKind::Cost, 1.0);
    r.max_rounds = 0;
    let v = repair(&store(ISSUE), &r).unwrap();
    assert_eq!(v["optimal"], false);
    assert_eq!(v["optimality"]["bound_fired"], "max_rounds");
    assert_eq!(v["coherence_after"]["verdict"], "coherent_in_this_fragment");
    assert!(render_human(&v).contains("NOT established"));
}

#[test]
fn the_out_file_is_coherent_when_defects_reads_it_back() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("repaired.ttl");
    let mut o = opts(WeightKind::Cost, 1.0);
    o.out = Some(out.clone());
    let v = repair(&store(ISSUE), &o).unwrap();
    assert_eq!(v["out"]["axioms_removed"], 1);
    assert_eq!(v["out"]["annotation_triples_removed"], 5, "the dropped axiom's reification goes too");
    assert_eq!(v["out"]["rechecked"]["incoherent_classes_in_this_fragment"], 0);

    // Read back independently of the tool's own re-check.
    let back = Arc::new(GraphStore::new());
    back.load_turtle(&std::fs::read_to_string(&out).unwrap(), None).unwrap();
    let errors: Vec<_> = Defects::findings(&back)
        .unwrap()
        .into_iter()
        .filter(|f| f.severity == "error")
        .collect();
    assert!(errors.is_empty(), "defects still finds {errors:?} in the repaired file");
    let triples = back.all_triples().unwrap();
    let has = |p: &str| {
        let want = (format!("<{}>", ex("A")), format!("<{p}>"), format!("<{}>", ex("B")));
        triples.contains(&want)
    };
    assert!(!has(DISJ), "the dropped axiom must be gone");
    assert!(has(SUB), "the kept axiom must survive");
    // The original store is not touched.
    let before = store(ISSUE);
    let n = before.triple_count();
    repair(&before, &o).unwrap();
    assert_eq!(before.triple_count(), n);
}

// ── The other two surfaces ─────────────────────────────────────────────────

fn cli(dir: &std::path::Path, args: &[&str], envs: &[(&str, &str)]) -> (i32, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_open-ontologies"));
    cmd.arg("--data-dir").arg(dir.join("data")).args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("the binary runs");
    (
        out.status.code().unwrap_or(-1),
        format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)),
    )
}

#[test]
fn the_cli_writes_a_file_defects_passes() {
    let d = tempfile::tempdir().unwrap();
    let input = d.path().join("issue.ttl");
    std::fs::write(&input, format!("{PREFIXES}{ISSUE}")).unwrap();
    let out = d.path().join("repaired.ttl");
    let (code, text) = cli(
        d.path(),
        &[
            "repair",
            input.to_str().unwrap(),
            "--weight-property",
            W,
            "--out",
            out.to_str().unwrap(),
        ],
        &[],
    );
    assert_eq!(code, 0, "{text}");
    let v: Value = serde_json::from_str(text.trim()).expect("JSON by default");
    assert_eq!(dropped(&v), set(&["A disjointWith B"]));

    let (code, text) = cli(d.path(), &["defects", out.to_str().unwrap()], &[]);
    assert_eq!(code, 0, "{text}");
    let report: Value = serde_json::from_str(text.trim()).unwrap();
    assert_eq!(report["affected_classes"], 0, "defects on the repaired file: {report}");

    let (code, text) = cli(d.path(), &["--human", "repair", input.to_str().unwrap(), "--weight-property", W], &[]);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("Weighted repair (cost weights): drop 1 axiom"), "{text}");

    let (code, text) = cli(d.path(), &["repair", input.to_str().unwrap(), "--weights", "confidence"], &[]);
    assert_eq!(code, 1, "an unknown weight kind is an error: {text}");
    let (code, text) = cli(d.path(), &["repair", d.path().join("missing.ttl").to_str().unwrap()], &[]);
    assert_eq!(code, 1, "a missing file is an error: {text}");
}

#[tokio::test]
async fn the_batch_verb_repairs_the_loaded_store() {
    use open_ontologies::batch::BatchRunner;
    use open_ontologies::state::StateDb;
    let d = tempfile::tempdir().unwrap();
    let input = d.path().join("issue.ttl");
    std::fs::write(&input, format!("{PREFIXES}{ISSUE}")).unwrap();
    let db = StateDb::open(&d.path().join("state.db")).unwrap();
    let runner = BatchRunner::new(db, Arc::new(GraphStore::new()), false);
    let script = serde_json::json!([
        {"command": "load", "args": [input.to_str().unwrap()]},
        {"command": "repair", "args": ["--weight-property", W]},
        {"command": "repair", "args": [input.to_str().unwrap(), "--weights", "probability",
                                        "--default-weight", "1", "--weight-property", W]},
    ]);
    let (results, code) = runner.run_collect(&script.to_string(), true).await;
    assert_eq!(code, 0, "{results:?}");
    assert_eq!(dropped(&results[1]["result"]), set(&["A disjointWith B"]));
    // Under probability the same file has a different objective, and the
    // log-odds of 0.62 (0.490) still undercut those of 0.87 (1.901).
    assert_eq!(dropped(&results[2]["result"]), set(&["A disjointWith B"]));
    assert_eq!(results[2]["result"]["weights"], "probability");
}

#[tokio::test]
async fn the_mcp_tool_repairs_the_loaded_store_and_leaves_it_alone() {
    use open_ontologies::inputs::OntoRepairInput;
    use open_ontologies::server::OpenOntologiesServer;
    use open_ontologies::state::StateDb;
    use rmcp::handler::server::wrapper::Parameters;
    let d = tempfile::tempdir().unwrap();
    let graph = store(ISSUE);
    let n = graph.triple_count();
    let server = OpenOntologiesServer::new_with_graph(StateDb::open(&d.path().join("s.db")).unwrap(), graph.clone());
    let text = server
        .onto_repair(Parameters(OntoRepairInput {
            weight_property: Some(format!("<{W}>")),
            ..Default::default()
        }))
        .await;
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(dropped(&v), set(&["A disjointWith B"]), "{v:#}");
    assert_eq!(graph.triple_count(), n, "the store is not changed");
    let bad = server
        .onto_repair(Parameters(OntoRepairInput { weights: Some("votes".into()), ..Default::default() }))
        .await;
    assert!(bad.contains("error"), "{bad}");
}
