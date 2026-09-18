//! **A load is all or nothing to a concurrent reader.**
//!
//! `load_turtle` already parsed the whole document before touching the store,
//! which made a load all-or-nothing against a SYNTAX error (#93). It was not
//! all-or-nothing against a concurrent READ: the inserts ran one quad at a
//! time, so another session walking the store could capture a half-loaded
//! ontology.
//!
//! That matters because of who the other session is. `serve-http` and `daemon`
//! share one `Arc<GraphStore>` across every session, and if the reader is a
//! certified run it writes an `asserted.tsv` describing whatever it saw. TCB-6
//! and TCB-7 say that file IS the graph reasoned over. The Lean checker cannot
//! see the difference: a half-loaded graph is internally consistent, so the
//! certificate would prove things about an ontology nobody ever had.
//!
//! `tests/store_atomicity_test.rs` measured the raw store behaviour. This
//! measures `GraphStore::load_turtle`, which is what callers actually use.

use open_ontologies::graph::GraphStore;
use std::collections::BTreeSet;
use std::sync::Arc;

/// Enough statements that a reader gets many chances to catch the load half
/// done, and small enough to stay quick.
const N: usize = 400;

fn document() -> String {
    let mut s = String::from("@prefix ex: <http://example.org/> .\n");
    for i in 0..N {
        s.push_str(&format!("ex:s{i} ex:p ex:o .\n"));
    }
    s
}

#[test]
fn a_reader_never_sees_a_load_part_way_through() {
    let graph = Arc::new(GraphStore::new());
    let writer = Arc::clone(&graph);
    let doc = document();

    let t = std::thread::spawn(move || {
        writer.load_turtle(&doc, None).expect("load");
    });

    // Poll until the load lands, recording every size seen on the way.
    let mut seen = BTreeSet::new();
    loop {
        let n = graph.all_triples().map(|t| t.len()).unwrap_or(0);
        seen.insert(n);
        if n >= N {
            break;
        }
    }
    t.join().expect("writer");

    let partial: Vec<&usize> = seen.iter().filter(|n| **n > 0 && **n < N).collect();
    assert!(
        partial.is_empty(),
        "a reader saw the store holding {partial:?} statements while a {N} statement document \
         was loading. A certified run reading at that moment would record an asserted.tsv for an \
         ontology that was never loaded, and the checker would accept it because the file is \
         internally consistent. Sizes seen: {seen:?}"
    );
    assert!(
        seen.contains(&N),
        "the load never completed, so this test measured nothing: {seen:?}"
    );
}

/// The all-or-nothing guarantee against a syntax error still holds. A
/// transaction that committed before the parse checked out would trade one
/// defect for the other.
#[test]
fn a_document_with_a_syntax_error_still_loads_nothing() {
    let graph = Arc::new(GraphStore::new());
    let mut doc = document();
    doc.push_str("ex:broken ex:p ex:o\nthis is not turtle .\n");
    assert!(graph.load_turtle(&doc, None).is_err(), "a malformed document must be refused");
    let n = graph.all_triples().map(|t| t.len()).unwrap_or(0);
    assert_eq!(n, 0, "a refused load must leave the store empty, found {n} statements");
}
