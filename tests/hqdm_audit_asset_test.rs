//! The HQDM audit figure states three counts. This recomputes all three from
//! the committed rows and requires the asset to agree.
//!
//! The rule is the repository's: never type a number next to the picture that
//! shows it. Every count in `docs/assets/hqdm-audit.svg` is produced by
//! `hqdm-audit.py` from `docs/assets/hqdm/asserted.tsv`, and this test derives
//! them again, independently, in a different language.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

const R: &str = "http://www.w3.org/2000/01/rdf-schema#";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rows() -> Vec<(String, String, String)> {
    let text = std::fs::read_to_string(repo().join("docs/assets/hqdm/asserted.tsv"))
        .expect("docs/assets/hqdm/asserted.tsv");
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

struct Found {
    undeclared: usize,
    bad_range: usize,
    twins: usize,
    identical: usize,
    declared: usize,
    terms: usize,
}

fn find() -> Found {
    let rows = rows();
    let (subclass, ty) = (format!("<{R}subClassOf>"), format!("<{RDF}type>"));
    let (domain, range) = (format!("<{R}domain>"), format!("<{R}range>"));
    let class = format!("<{R}Class>");

    let declared: HashSet<&String> = rows
        .iter()
        .filter(|(_, p, o)| *p == ty && *o == class)
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
        .filter(|t| !declared.contains(**t) && t.starts_with('<'))
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

    let subjects: HashSet<&String> = rows.iter().map(|(s, _, _)| s).collect();
    let mut facts: HashMap<&String, HashSet<(&String, &String)>> = HashMap::new();
    for (s, p, o) in &rows {
        facts.entry(s).or_default().insert((p, o));
    }
    let (mut twins, mut identical) = (0, 0);
    for a in &subjects {
        let b = if a.ends_with('>') {
            format!("{}_>", &a[..a.len() - 1])
        } else {
            format!("{a}_")
        };
        if let Some(bkey) = subjects.get(&b) {
            twins += 1;
            if facts.get(*a) == facts.get(*bkey) {
                identical += 1;
            }
        }
    }

    let linking = [&subclass, &ty, &domain, &range];
    let terms: HashSet<&String> = rows
        .iter()
        .filter(|(_, p, o)| linking.contains(&p) && o.starts_with('<'))
        .flat_map(|(s, _, o)| [s, o])
        .collect();

    Found {
        undeclared: undeclared.len(),
        bad_range,
        twins,
        identical,
        declared: declared.len(),
        terms: terms.len(),
    }
}

#[test]
fn the_audit_states_the_counts_the_rows_produce() {
    let f = find();
    let s = svg();
    // Each count appears in the running caption AND in the legend, so a stale
    // one has to be stale in both places to slip past.
    for (n, what) in [
        (f.undeclared, "terms are used as a class and never declared"),
        (f.bad_range, "rdfs:range declarations name a relation, not a class"),
    ] {
        let want = format!("{n} {what}");
        assert!(
            s.contains(&want),
            "the asset does not say {want:?}. Regenerate it from the rows: \
             python3 docs/assets/hqdm-audit.py docs/assets/hqdm/asserted.tsv \
             docs/assets/hqdm-audit.svg"
        );
    }
    assert!(
        s.contains(&format!("{} names differ only by a trailing underscore, {} of them identical",
                            f.twins, f.identical)),
        "the underscore-twin counts in the asset are not the {} / {} the rows give",
        f.twins, f.identical
    );
    assert!(
        s.contains(&format!("{} declared classes, {} terms in one", f.declared, f.terms)),
        "the asset does not state {} declared classes and {} terms",
        f.declared, f.terms
    );
}

#[test]
fn the_audit_does_not_claim_a_certificate() {
    // Nothing here was proved. The reasoning layer never ran on this file, and
    // the three findings are structural. A word from the certificate vocabulary
    // in this asset would be claiming a warrant that does not exist.
    let s = svg();
    for word in ["certified", "certificate", "proved", "entailed", "model_checked"] {
        assert!(
            !s.contains(word),
            "the HQDM audit says {word:?}. No checker ran on this file; these \
             findings are structural and must not borrow the proof vocabulary."
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
