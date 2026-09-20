//! The HQDM audit figure draws two shipped renderings of one ontology and
//! states counts for both. This recomputes every one of them from the committed
//! inputs and requires the asset, both READMEs and the engine to agree.
//!
//! The rule is the repository's: never type a number next to the picture that
//! shows it. Every count in `docs/assets/hqdm-audit.svg` is produced by
//! `hqdm-audit.py` from the files under `docs/assets/hqdm/`, and this test
//! derives them again, independently, in a different language. The one number
//! that comes from a reasoner, the 39 classes the tableaux leaves undecided, is
//! recomputed by running the tableaux, not by reading the JSON back.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

const R: &str = "http://www.w3.org/2000/01/rdf-schema#";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const OWL: &str = "http://www.w3.org/2002/07/owl#";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const HQDM: &str = "http://www.semanticweb.org/magma-core/ontologies/hqdm#";

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rows(file: &str) -> Vec<(String, String, String)> {
    let path = repo().join("docs/assets/hqdm").join(file);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.lines()
        .filter_map(|l| {
            let p: Vec<&str> = l.split('\t').collect();
            (p.len() >= 3).then(|| (p[0].to_string(), p[1].to_string(), p[2].to_string()))
        })
        .collect()
}

fn svg() -> String {
    std::fs::read_to_string(repo().join("docs/assets/hqdm-audit.svg"))
        .expect("docs/assets/hqdm-audit.svg")
}

/// The engine's committed run over `hqdm.owl`.
fn owl_dl() -> serde_json::Value {
    serde_json::from_str(
        &std::fs::read_to_string(repo().join("docs/assets/hqdm/owl-dl.json"))
            .expect("docs/assets/hqdm/owl-dl.json"),
    )
    .expect("owl-dl.json is JSON")
}

fn hermit() -> HashSet<String> {
    std::fs::read_to_string(repo().join("docs/assets/hqdm/hermit-unsatisfiable.txt"))
        .expect("docs/assets/hqdm/hermit-unsatisfiable.txt")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| format!("<{HQDM}{}>", l.trim()))
        .collect()
}

fn names(v: &serde_json::Value) -> HashSet<String> {
    v.as_array()
        .expect("a list of classes")
        .iter()
        .map(|x| x.as_str().expect("a class name").to_string())
        .collect()
}

struct Found {
    undeclared: usize,
    bad_range: usize,
    twins: usize,
    identical: usize,
    declared: usize,
    terms: usize,
    triples: usize,
}

/// A datatype in range position is correct RDFS, not a missing class. The
/// RDFS rendering has none, so this changes nothing there.
fn is_datatype(t: &str) -> bool {
    let b = t.trim_matches(|c| c == '<' || c == '>');
    b.starts_with(XSD)
        || [
            format!("{R}Literal"),
            format!("{RDF}PlainLiteral"),
            format!("{RDF}langString"),
            format!("{OWL}rational"),
            format!("{OWL}real"),
        ]
        .iter()
        .any(|x| x == b)
}

fn find(file: &str) -> Found {
    let rows = rows(file);
    let (subclass, ty) = (format!("<{R}subClassOf>"), format!("<{RDF}type>"));
    let (domain, range) = (format!("<{R}domain>"), format!("<{R}range>"));
    let (disjoint, subprop) = (format!("<{OWL}disjointWith>"), format!("<{R}subPropertyOf>"));
    let classes = [format!("<{R}Class>"), format!("<{OWL}Class>")];
    let meta: HashSet<String> = [
        "Class", "ObjectProperty", "DatatypeProperty", "Ontology", "NamedIndividual",
        "Restriction", "TransitiveProperty", "FunctionalProperty",
    ]
    .iter()
    .map(|x| format!("<{OWL}{x}>"))
    .chain(std::iter::once(format!("<{R}Class>")))
    .collect();

    let declared: HashSet<&String> = rows
        .iter()
        .filter(|(_, p, o)| *p == ty && classes.contains(o))
        .map(|(s, _, _)| s)
        .collect();

    let mut used: HashSet<&String> = HashSet::new();
    for (s, p, o) in &rows {
        if *p == subclass {
            used.insert(s);
            used.insert(o);
        } else if *p == domain || *p == range {
            used.insert(o);
        }
    }
    let undeclared: HashSet<&&String> = used
        .iter()
        .filter(|t| !declared.contains(**t) && t.starts_with('<') && !is_datatype(t))
        .collect();

    let hierarchy: HashSet<&String> = rows
        .iter()
        .filter(|(_, p, _)| *p == subclass)
        .flat_map(|(s, _, o)| [s, o])
        .collect();
    let not_a_class: HashSet<&String> = undeclared
        .iter()
        .filter(|t| !hierarchy.contains(***t))
        .map(|t| **t)
        .collect();
    let bad_range = rows
        .iter()
        .filter(|(_, p, o)| *p == range && not_a_class.contains(o))
        .count();

    let subjects: HashSet<&String> =
        rows.iter().map(|(s, _, _)| s).filter(|s| s.starts_with('<')).collect();
    let mut facts: HashMap<&String, HashSet<(&String, &String)>> = HashMap::new();
    for (s, p, o) in &rows {
        facts.entry(s).or_default().insert((p, o));
    }
    let (mut twins, mut identical) = (0, 0);
    for a in &subjects {
        let b = format!("{}_>", &a[..a.len() - 1]);
        if let Some(bkey) = subjects.get(&b) {
            twins += 1;
            if facts.get(*a) == facts.get(*bkey) {
                identical += 1;
            }
        }
    }

    // The drawn graph: IRI-to-IRI edges under the linking predicates, minus
    // the vocabulary hubs. Same rule as the generator, in a different language.
    let linking = [&subclass, &ty, &domain, &range, &disjoint, &subprop];
    let terms: HashSet<&String> = rows
        .iter()
        .filter(|(s, p, o)| {
            linking.contains(&p) && s.starts_with('<') && o.starts_with('<') && !meta.contains(o)
        })
        .flat_map(|(s, _, o)| [s, o])
        .collect();

    Found {
        undeclared: undeclared.len(),
        bad_range,
        twins,
        identical,
        declared: declared.len(),
        terms: terms.len(),
        triples: rows.len(),
    }
}

#[test]
fn the_rdfs_rendering_states_the_counts_the_rows_produce() {
    let f = find("asserted.tsv");
    let s = svg();
    for want in [
        format!("left: {} terms are used as a class and never declared", f.undeclared),
        format!("left: {} rdfs:range declarations name a relation, not a class", f.bad_range),
        format!(
            "{} triples, {} declared classes, {} terms in one connected graph",
            f.triples, f.declared, f.terms
        ),
        format!(
            "one trailing underscore apart; {} identical in domain and range",
            f.identical
        ),
    ] {
        assert!(
            s.contains(&want),
            "the asset does not say {want:?}. Regenerate it: python3 docs/assets/hqdm-audit.py \
             docs/assets/hqdm/asserted.tsv docs/assets/hqdm/owl-rows.tsv \
             docs/assets/hqdm/owl-dl.json docs/assets/hqdm/hermit-unsatisfiable.txt \
             docs/assets/hqdm-audit.svg"
        );
    }
    // The RDFS file is the one that MUST have nothing a reasoner can see.
    let owl_terms = rows("asserted.tsv")
        .iter()
        .filter(|(_, p, o)| p.starts_with(&format!("<{OWL}")) || o.starts_with(&format!("<{OWL}")))
        .count();
    assert_eq!(owl_terms, 0, "the RDFS rendering now carries owl: terms; the caption is wrong");
}

#[test]
fn the_owl_rendering_states_the_counts_the_rows_produce() {
    let f = find("owl-rows.tsv");
    let s = svg();
    for want in [
        format!(
            "{} triples, {} declared classes, {} terms in one connected graph",
            f.triples, f.declared, f.terms
        ),
        format!(
            "{} undeclared terms, {} ranges naming a relation: well formed by the same checks",
            f.undeclared, f.bad_range
        ),
    ] {
        assert!(s.contains(&want), "the asset does not say {want:?}");
    }
    let t = find("asserted.tsv");
    assert!(
        s.contains(&format!(
            "both: {} and {} names differ only by a trailing underscore",
            t.twins, f.twins
        )),
        "the underscore-twin counts are not the {} / {} the two files give",
        t.twins, f.twins
    );
    let disjoint = rows("owl-rows.tsv")
        .iter()
        .filter(|(_, p, _)| *p == format!("<{OWL}disjointWith>"))
        .count();
    assert!(s.contains(&format!("{disjoint} disjointness axioms")));
}

/// The figure's reasoner numbers come from the committed run and the committed
/// oracle list, and the intersection it prints is computed, not typed.
#[test]
fn the_reasoner_counts_and_the_intersection_are_derived() {
    let run = owl_dl();
    let undecided = names(&run["undetermined_classes"]);
    let refuted = names(&run["unsatisfiable_classes"]);
    let sat = run["agents"]["satisfiability_agent"]["satisfiable_found"]
        .as_u64()
        .expect("satisfiable_found") as usize;
    let checked = run["agents"]["satisfiability_agent"]["classes_checked"]
        .as_u64()
        .expect("classes_checked") as usize;
    assert_eq!(sat + undecided.len() + refuted.len(), checked, "the run does not add up");
    assert_eq!(run["complete"], false, "an incomplete run is what the figure describes");

    let oracle = hermit();
    let both = undecided.intersection(&oracle).count();
    let s = svg();
    for want in [
        format!("this engine finds {sat} named classes satisfiable and cannot decide {}", undecided.len()),
        format!(
            "HermiT, an opinion here, calls {} unsatisfiable, and they are the same {both}",
            oracle.len()
        ),
        format!("{both} of them are the undecided ones"),
    ] {
        assert!(s.contains(&want), "the asset does not say {want:?}");
    }
    // A gate must be able to fail: the intersection is asserted against the
    // sets, so a stale list on either side changes the number.
    assert_eq!(both, oracle.len(), "the oracle names a class the engine decided");
    assert_eq!(both, undecided.len(), "the engine left a class undecided that the oracle passed");
}

/// The committed run is reproducible: the tableaux leaves the SAME classes
/// undecided today. This is the one place a number in the figure is recomputed
/// by running the engine rather than counting rows.
///
/// The budget here is five times the one the committed run used. The 39
/// undecided classes stop on an expansion cap, not on the clock: the committed
/// run's satisfiability phase spent 1,068 ms of a 120,000 ms budget. A larger
/// budget therefore cannot decide them, and it protects the 195 satisfiable
/// ones from a slow runner turning a debug-build search into a timeout.
#[test]
fn the_reasoner_still_leaves_the_same_classes_undecided() {
    use open_ontologies::graph::GraphStore;
    use open_ontologies::tableaux::DlReasoner;
    use std::sync::Arc;

    let before = open_ontologies::runtime::tableaux_test_timeout_ms();
    open_ontologies::runtime::set_tableaux_test_timeout_ms(600_000);
    let store = Arc::new(GraphStore::new());
    let n = store
        .load_file(repo().join("docs/assets/hqdm/hqdm.owl").to_str().unwrap())
        .expect("hqdm.owl loads as RDF/XML");
    let out = DlReasoner::run(&store, false).expect("the tableaux runs");
    open_ontologies::runtime::set_tableaux_test_timeout_ms(before.unwrap_or(0) as usize);

    let live: serde_json::Value = serde_json::from_str(&out).unwrap();
    let committed = owl_dl();
    assert_eq!(n, rows("owl-rows.tsv").len(), "hqdm.owl and owl-rows.tsv differ in triple count");
    assert_eq!(
        names(&live["unsatisfiable_classes"]),
        names(&committed["unsatisfiable_classes"]),
        "the tableaux now refutes classes the committed run did not; regenerate owl-dl.json"
    );
    assert_eq!(
        names(&live["undetermined_classes"]),
        names(&committed["undetermined_classes"]),
        "the tableaux leaves a different set undecided than the committed run; regenerate owl-dl.json"
    );
}

#[test]
fn the_audit_does_not_claim_a_certificate() {
    // Nothing here was proved. The tableaux's `satisfiable` is a search result
    // with no model certificate behind it, HermiT's verdict is an opinion, and
    // the three findings are structural. A word from the certificate
    // vocabulary in this asset would be claiming a warrant that does not exist.
    let s = svg();
    for word in ["certified", "certificate", "proved", "entailed", "model_checked", "refuted"] {
        assert!(
            !s.contains(word),
            "the HQDM audit says {word:?}. No checker ran on these files; the \
             findings must not borrow the proof vocabulary."
        );
    }
}

#[test]
fn the_audit_is_well_formed_markup() {
    let s = svg();
    let opens = s.matches("<g ").count() + s.matches("<g>").count();
    let closes = s.matches("</g>").count();
    assert_eq!(opens, closes, "unbalanced <g> in the HQDM audit asset");
    assert!(s.starts_with("<svg") && s.ends_with("</svg>"));
}

/// The translated README must state the same numbers as the English one.
///
/// `README.zh-CN.md` carries the same two figures, and its captions repeat the
/// counts. A translation is the easiest place in a repository for a number to
/// go stale, because the person updating the English caption does not read the
/// other file. This requires both to agree, without needing to read Chinese:
/// the counts are digits in both.
#[test]
fn the_translated_readme_states_the_same_counts() {
    let f = find("asserted.tsv");
    let run = owl_dl();
    let sat = run["agents"]["satisfiability_agent"]["satisfiable_found"].as_u64().unwrap() as usize;
    let undecided = names(&run["undetermined_classes"]).len();
    let zh = std::fs::read_to_string(repo().join("README.zh-CN.md")).expect("README.zh-CN.md");
    let en = std::fs::read_to_string(repo().join("README.md")).expect("README.md");

    for (name, url) in [
        ("the certificate figure", "knowledge-graph.svg"),
        ("the HQDM audit", "hqdm-audit.svg"),
    ] {
        assert!(
            zh.contains(url),
            "README.zh-CN.md does not carry {name}. Both READMEs show the same \
             front page, or the translation is a different product."
        );
    }

    // The HQDM counts, recomputed from the rows and the run, must appear in BOTH.
    for n in [f.undeclared, f.bad_range, f.twins, sat, undecided] {
        let needle = format!("<b>{n}</b>");
        assert!(en.contains(&needle), "README.md does not state {n} for the HQDM audit");
        assert!(
            zh.contains(&needle),
            "README.zh-CN.md does not state {n} for the HQDM audit, and the \
             English one does. A count that moved on one side only is the way \
             a translation starts lying."
        );
    }
    for readme in [&en, &zh] {
        assert!(
            !readme.contains("OM-2026 measured") && !readme.contains("not the file"),
            "the caption still carries the disclaimer the second panel replaced"
        );
    }

    // And the legend counts of the certificate figure, whatever they are today.
    let svg = std::fs::read_to_string(repo().join("docs/assets/knowledge-graph.svg"))
        .expect("docs/assets/knowledge-graph.svg");
    for word in ["ASSERTED", "CERTIFIED", "REJECTED"] {
        let at = svg.find(word).unwrap_or_else(|| panic!("the legend lost {word}"));
        let after = &svg[at..];
        let n: String = after
            .split('>')
            .find_map(|seg| {
                let d: String = seg.chars().take_while(|c| c.is_ascii_digit()).collect();
                (!d.is_empty()).then_some(d)
            })
            .expect("a number follows the legend word");
        assert!(
            zh.contains(&n),
            "the asset's legend says {n} for {word} and README.zh-CN.md does not \
             mention it"
        );
    }
}
