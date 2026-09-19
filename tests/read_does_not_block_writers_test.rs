//! **A certified read no longer stops the store (#181).**
//!
//! `triples_outside` is the selection the certified reasoning paths use, and it
//! has to see ONE state: a torn read produces an `asserted.tsv` that is
//! internally consistent and describes a graph nobody ever had, which the Lean
//! checker cannot detect because it never sees the store. #195 made it atomic by
//! wrapping it in a transaction, and that was right about the atomicity and
//! expensive in a way nobody had measured.
//!
//! On the in-memory backend, which is what `GraphStore::new` builds and what
//! `serve-http` and `daemon` share across every session, an open read
//! transaction blocks EVERY writer until it commits, because `Store::insert`
//! opens a transaction of its own and `MemoryStorage::start_transaction` waits.
//! A selection over a large graph therefore stopped the whole store for the
//! length of the read.
//!
//! It now takes a snapshot instead: the same one state, no lock. This file is
//! the gate on that, and it is written so that it fails if the transaction ever
//! comes back, rather than merely passing faster.

use open_ontologies::graph::GraphStore;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const BIG: usize = 200_000;

fn turtle(from: usize, n: usize) -> String {
    let mut s = String::with_capacity(n * 60);
    for i in from..from + n {
        s.push_str(&format!(
            "<http://ex.org/{i}> <http://ex.org/p> <http://ex.org/o> .\n"
        ));
    }
    s
}

#[test]
fn a_writer_makes_progress_while_a_long_certified_read_runs() {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(&turtle(0, BIG), None).expect("seed");

    let running = Arc::new(AtomicBool::new(true));
    let writes = Arc::new(AtomicUsize::new(0));
    let (w, wr, wc) = (Arc::clone(&graph), Arc::clone(&running), Arc::clone(&writes));
    let writer = std::thread::spawn(move || {
        let mut i = 10_000_000usize;
        while wr.load(Ordering::SeqCst) {
            w.load_turtle(&turtle(i, 1), None).expect("write");
            wc.fetch_add(1, Ordering::SeqCst);
            i += 1;
        }
    });

    // Let the writer get going, so a zero below means blocked rather than
    // not yet started.
    let warmup_deadline = Instant::now() + Duration::from_secs(5);
    while writes.load(Ordering::SeqCst) < 5 && Instant::now() < warmup_deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    let warm = writes.load(Ordering::SeqCst);
    assert!(warm >= 5, "the writer never started; it managed {warm} writes before the read");

    // ONE read. The whole measurement is what the writer achieves DURING it,
    // because a transaction is released between calls and a test that made
    // several reads would hand the writer a gap after each one and measure
    // nothing. That is exactly what the first version of this test did, and it
    // passed with the transaction restored.
    let before = writes.load(Ordering::SeqCst);
    let t0 = Instant::now();
    let (triples, _graphs) = graph.triples_outside(&[]).expect("read");
    let read = t0.elapsed();
    let during = writes.load(Ordering::SeqCst) - before;

    running.store(false, Ordering::SeqCst);
    writer.join().expect("writer thread");

    assert!(
        triples.len() >= BIG,
        "the selection returned {} triples, fewer than the {BIG} seeded",
        triples.len()
    );
    assert!(
        read > Duration::from_millis(100),
        "a single read took {read:?}, too short for this test to mean anything. Raise BIG \
         rather than deleting the assertion."
    );
    assert!(
        during > 0,
        "the writer completed NOTHING during a single {read:?} read, having managed {warm} \
         writes in the moments before it. That is what a read TRANSACTION does on the \
         in-memory backend: the writer cannot open its own transaction until the reader \
         commits. The snapshot has been reverted."
    );
}
