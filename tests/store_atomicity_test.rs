//! **A reader can see a half-written graph, and a transaction stops it.**
//!
//! `docs/decisions/0010-the-input-is-a-value-and-not-a-store.md` says "no
//! transactional snapshot is available to lean on. Copying is the only honest
//! option available." The second sentence follows from the first, and the
//! first is not true of the Oxigraph this repository pins: `oxigraph = "0.5"`,
//! whose `Store::start_transaction` documents the "repeatable read" isolation
//! level and atomic transactional operations. Issue #181.
//!
//! This measures the difference rather than quoting the documentation.
//!
//! | writer | partial reads a concurrent reader saw |
//! | --- | ---: |
//! | one quad at a time, which is what the loaders do | many |
//! | one transaction, committed at the end | none |
//!
//! ## Why this is about certificates and not about tidiness
//!
//! TCB-6 and TCB-7 say `asserted.tsv` IS the graph that was reasoned over.
//! `docs/trusted-computing-base.md` calls them irreducible because they are
//! claims about an execution rather than about a function. A torn read is
//! exactly how they break: in `serve-http` and `daemon` mode one
//! `Arc<GraphStore>` is shared by every session, so a certified run that reads
//! while another session loads can write an `asserted.tsv` describing a graph
//! that existed at no instant. The Lean checker cannot see that. It reads the
//! file it is given and proves things about it, and the file would be a true
//! record of a graph nobody ever had.
//!
//! That is the same shape as the defect #108 closed for named graphs, arriving
//! through concurrency instead of through scope.
//!
//! ## What this test does NOT claim
//!
//! It does not claim the engine is currently wrong. Every certified path today
//! runs in a context this test does not model, and whether a concurrent write
//! can actually land mid-run is a question about the callers. It claims the
//! store permits a torn read and that a transaction removes it, which is the
//! fact decision 0010 assumed the other way round.

use oxigraph::model::{GraphNameRef, NamedNodeRef, QuadRef};
use oxigraph::store::Store;
use std::collections::BTreeSet;
use std::sync::Arc;

/// Big enough that a reader gets many chances to catch the write half done,
/// small enough that the test stays quick. The writer sleeps between quads for
/// the same reason: without it the write finishes before the reader starts and
/// the test passes having raced nothing, which is how the first version of
/// this probe reported "no partial reads" from both arms.
const BATCH: usize = 200;
const PACE: std::time::Duration = std::time::Duration::from_micros(200);

fn quad_subject(i: usize) -> String {
    format!("http://example.org/s{i}")
}

/// Read repeatedly until the batch has fully landed, recording every distinct
/// size seen on the way.
fn watch(store: &Store) -> BTreeSet<usize> {
    let mut seen = BTreeSet::new();
    loop {
        let n = store.iter().count();
        seen.insert(n);
        if n >= BATCH {
            return seen;
        }
    }
}

#[test]
fn a_reader_sees_a_partly_written_graph_when_the_writer_uses_no_transaction() {
    let store = Arc::new(Store::new().expect("store"));
    let writer = Arc::clone(&store);
    let t = std::thread::spawn(move || {
        let p = NamedNodeRef::new("http://example.org/p").expect("p");
        let o = NamedNodeRef::new("http://example.org/o").expect("o");
        for i in 0..BATCH {
            let s = quad_subject(i);
            let sr = NamedNodeRef::new(&s).expect("s");
            writer
                .insert(QuadRef::new(sr, p, o, GraphNameRef::DefaultGraph))
                .expect("insert");
            std::thread::sleep(PACE);
        }
    });

    let seen = watch(&store);
    t.join().expect("writer");

    let partial: Vec<&usize> = seen.iter().filter(|n| **n > 0 && **n < BATCH).collect();
    assert!(
        !partial.is_empty(),
        "the reader never caught the write half done, so this test raced nothing and the \
         companion test below proves nothing by contrast. Sizes seen: {seen:?}"
    );
}

#[test]
fn a_transaction_makes_the_same_write_all_or_nothing() {
    let store = Arc::new(Store::new().expect("store"));
    let writer = Arc::clone(&store);
    let t = std::thread::spawn(move || {
        let mut txn = writer.start_transaction().expect("begin");
        let p = NamedNodeRef::new("http://example.org/p").expect("p");
        let o = NamedNodeRef::new("http://example.org/o").expect("o");
        for i in 0..BATCH {
            let s = quad_subject(i);
            let sr = NamedNodeRef::new(&s).expect("s");
            txn.insert(QuadRef::new(sr, p, o, GraphNameRef::DefaultGraph));
            std::thread::sleep(PACE);
        }
        txn.commit().expect("commit");
    });

    let seen = watch(&store);
    t.join().expect("writer");

    assert_eq!(
        seen,
        BTreeSet::from([0, BATCH]),
        "a reader must see the graph before the batch or after it and never inside it, so the \
         only sizes are 0 and {BATCH}. Seeing anything else means the transaction did not give \
         the atomicity decision 0010 assumed was unavailable, and that assumption would then be \
         right for a different reason than the one it states"
    );
}
