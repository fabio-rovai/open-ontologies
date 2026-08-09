//! Latency of the compiled claim check on the REAL Pizza ontology.
//!
//! Loads the output of `CompileOntology.java` (`/tmp/pizza_compiled.json`) so
//! this measures the real inferred hierarchy and real disjointness axioms, not
//! a hand-written fixture. Skips cleanly when that file is absent, so it does
//! not break a checkout that has not run the Java compile step.
//!
//! Regenerate with:
//!   java -cp ".:lib/*" CompileOntology /tmp/pizza_real.owl /tmp/pizza_compiled.json

use std::path::Path;

use open_ontologies::claimcheck::{Claim, CompiledOntology, Verdict};
use serde_json::Value;

const COMPILED: &str = "/tmp/pizza_compiled.json";
const PIZZA: &str = "http://www.co-ode.org/ontologies/pizza/pizza.owl#";

fn iri(l: &str) -> String {
    format!("{PIZZA}{l}")
}

fn load() -> Option<CompiledOntology> {
    if !Path::new(COMPILED).exists() {
        eprintln!("SKIP: {COMPILED} not present; run CompileOntology first");
        return None;
    }
    let v: Value = serde_json::from_str(&std::fs::read_to_string(COMPILED).ok()?).ok()?;
    let c = CompiledOntology::new().ok()?;

    let pairs = |key: &str| -> Vec<(String, String)> {
        v[key]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|p| {
                        Some((
                            p.get(0)?.as_str()?.to_string(),
                            p.get(1)?.as_str()?.to_string(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    let subs = pairs("subsumptions");
    let disj = pairs("disjoint");
    // Everything mentioned anywhere counts as declared, so the vocabulary
    // check does not fire spuriously in this benchmark.
    let mut classes: Vec<String> = subs
        .iter()
        .flat_map(|(a, b)| [a.clone(), b.clone()])
        .chain(disj.iter().flat_map(|(a, b)| [a.clone(), b.clone()]))
        .collect();
    classes.sort();
    classes.dedup();

    c.load_subsumptions(&subs).ok()?;
    c.load_disjoint(&disj).ok()?;
    c.load_declared_classes(&classes).ok()?;
    eprintln!(
        "loaded {} subsumptions, {} disjoint axioms, {} classes",
        subs.len(),
        disj.len(),
        classes.len()
    );
    Some(c)
}

#[test]
fn real_pizza_verdicts_and_latency() {
    let Some(c) = load() else { return };

    // Ground truth established earlier against a brute-force HermiT matrix.
    let cases: &[(&str, &str, bool)] = &[
        ("Margherita", "MeatyPizza", true),
        ("American", "AmericanHot", true),
        ("Margherita", "CheeseyPizza", false),
        ("American", "MeatyPizza", false),
    ];

    for (a, b, expect_reject) in cases {
        let claim = Claim {
            types: vec![("x".into(), iri(a)), ("x".into(), iri(b))],
            ..Default::default()
        };
        let r = c.check(&claim).unwrap();
        let rejected = r.verdict == Verdict::Rejected;
        assert_eq!(
            rejected, *expect_reject,
            "{a} + {b}: got {:?}, expected reject={expect_reject}",
            r.verdict
        );
    }

    // Latency over a mixed stream.
    let claims: Vec<Claim> = cases
        .iter()
        .map(|(a, b, _)| Claim {
            types: vec![("x".into(), iri(a)), ("x".into(), iri(b))],
            ..Default::default()
        })
        .collect();

    let mut lat: Vec<u128> = Vec::with_capacity(4000);
    for i in 0..4000 {
        let t = std::time::Instant::now();
        let _ = c.check(&claims[i % claims.len()]).unwrap();
        lat.push(t.elapsed().as_nanos());
    }
    lat.sort_unstable();
    let us = |n: u128| n as f64 / 1000.0;
    eprintln!(
        "per-claim: median {:.1} us | p95 {:.1} us | p99 {:.1} us | max {:.1} us",
        us(lat[lat.len() / 2]),
        us(lat[lat.len() * 95 / 100]),
        us(lat[lat.len() * 99 / 100]),
        us(lat[lat.len() - 1])
    );

    // The 500 ms pipeline budget is 500,000 us. Anything near it is a defect.
    assert!(
        us(lat[lat.len() * 95 / 100]) < 1000.0,
        "p95 exceeded 1 ms, which would be a regression against the DuckDB baseline"
    );

    // Throughput: sequential loop vs rayon batch over the same mixed stream.
    let big: Vec<Claim> = (0..200_000).map(|i| claims[i % claims.len()].clone()).collect();

    let t = std::time::Instant::now();
    for cl in &big {
        let _ = c.check(cl).unwrap();
    }
    let seq = t.elapsed();

    let t = std::time::Instant::now();
    let out = c.check_batch(&big).unwrap();
    let par = t.elapsed();
    assert_eq!(out.len(), big.len());

    let seq_rate = big.len() as f64 / seq.as_secs_f64();
    let par_rate = big.len() as f64 / par.as_secs_f64();
    eprintln!(
        "throughput: sequential {:.0} claims/s | batch(rayon) {:.0} claims/s | scaling {:.1}x",
        seq_rate, par_rate, par_rate / seq_rate
    );
}
