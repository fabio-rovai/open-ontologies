//! **What a read transaction costs in memory, against what the copy costs (#181).**
//!
//! Oxigraph warns that "the transaction keeps the complete set of changes into
//! memory, do not use them to load tens of millions of triples". That is a
//! warning about WRITE volume, and `docs/decisions/0010` is about a read. The
//! question it leaves open is whether a read-only transaction carries a cost of
//! its own, and how that compares with the copy 0010 makes instead.
//!
//! Measured here with a counting allocator rather than reasoned about, because
//! the answer decides which of two costs the decision is choosing between.
//!
//! This is its own test binary on purpose: `#[global_allocator]` is per-binary,
//! and putting one in a file that also runs the concurrency probes would have
//! the probes' threads counted as part of the measurement.

use oxigraph::model::{GraphNameRef, NamedNodeRef, QuadRef};
use oxigraph::store::Store;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        LIVE.fetch_add(l.size(), Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Ordering::Relaxed);
        unsafe { System.dealloc(p, l) }
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

fn live() -> usize {
    LIVE.load(Ordering::Relaxed)
}

const N: usize = 20_000;

fn seed(store: &Store, n: usize) {
    let p = NamedNodeRef::new("http://ex.org/p").unwrap();
    let o = NamedNodeRef::new("http://ex.org/o").unwrap();
    for i in 0..n {
        let s = format!("http://ex.org/{i}");
        store
            .insert(QuadRef::new(
                NamedNodeRef::new(&s).unwrap(),
                p,
                o,
                GraphNameRef::DefaultGraph,
            ))
            .unwrap();
    }
}

/// The comparison decision 0010 is actually making: hold a transaction open
/// while you read, or copy the triples out and let go.
#[test]
fn a_read_only_transaction_costs_far_less_memory_than_the_copy() {
    let store = Store::new().unwrap();
    seed(&store, N);

    // (a) Hold a transaction and scan it. Nothing is written, so the change
    // set the vendor's warning is about should stay empty.
    let base = live();
    let txn = store.start_transaction().unwrap();
    let mut seen = 0usize;
    for q in txn.quads_for_pattern(None, None, None, None) {
        let _ = q.unwrap();
        seen += 1;
    }
    let held = live().saturating_sub(base);
    txn.commit().unwrap();
    assert_eq!(seen, N, "the scan did not see the whole store");

    // (b) The copy: the owned value 0010 builds, as three Strings per triple,
    // which is the shape `GraphStore::all_triples` produces.
    let base2 = live();
    let mut owned: Vec<(String, String, String)> = Vec::new();
    for q in store.iter() {
        let q = q.unwrap();
        owned.push((
            q.subject.to_string(),
            q.predicate.to_string(),
            q.object.to_string(),
        ));
    }
    let copied = live().saturating_sub(base2);
    assert_eq!(owned.len(), N);

    println!("triples                     {N}");
    println!("held open, read-only txn    {held} bytes");
    println!("copied out, owned value     {copied} bytes");
    println!("ratio copy:txn              {:.1}x", copied as f64 / held.max(1) as f64);

    assert!(
        copied > held * 4,
        "the copy ({copied} bytes) is not materially more expensive than holding a read-only \
         transaction ({held} bytes) over {N} triples. If that is now true, decision 0010 is \
         paying memory for no isolation it could not get more cheaply, and the trade in it \
         should be re-argued rather than this assertion relaxed."
    );
    // The point of the first number: a read-only transaction does not
    // accumulate a change set, so the vendor's warning is about writes.
    assert!(
        held < copied / 4,
        "a read-only transaction held {held} bytes over {N} triples, which is not the flat \
         cost the vendor's warning implies for a transaction that writes nothing"
    );
}
