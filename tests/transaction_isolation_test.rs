//! **What Oxigraph 0.5's transactions actually give this repository (#181).**
//!
//! `docs/decisions/0010-the-input-is-a-value-and-not-a-store.md` justifies
//! copying the whole selected graph for every certified run on this premise:
//!
//! > `GraphStore::snapshot` is a serialiser that takes a format string, not an
//! > isolation primitive, and no transactional snapshot is available to lean on.
//!
//! The second half is false of the version pinned here. `Store::start_transaction`
//! is public and documented as repeatable read. A decision record resting on a
//! false claim about its own dependency is worth correcting whether or not the
//! decision changes.
//!
//! It does not change, and this file is why. The measurements below say that a
//! transaction is a usable snapshot and that taking one costs something the
//! decision record never weighed, because the two storage backends do not
//! behave alike:
//!
//!   * **In memory**, which is what `GraphStore::new` builds and what almost
//!     everything in this codebase uses, an open read transaction BLOCKS every
//!     writer. Not "may see stale data": a concurrent `insert` does not return
//!     until the reader commits, because `Store::insert` opens a transaction of
//!     its own and `MemoryStorage::start_transaction` waits.
//!   * **On disk**, a concurrent writer proceeds and the reader's view stays
//!     stable, which is what repeatable read is supposed to feel like.
//!
//! So the copy in 0010 is not working around a missing primitive. It is trading
//! one short exclusive window for a long one: copy the bytes and let go, rather
//! than hold the store against all writers for the length of a reasoning run.
//! That is a better reason than the one in the file, and it is measured here
//! rather than asserted there.

use oxigraph::model::{GraphNameRef, NamedNodeRef, QuadRef};
use oxigraph::store::Store;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Long enough that a writer which is merely slow finishes, short enough that a
/// writer which is blocked is obvious. The in-memory case below blocks for as
/// long as the transaction is held, so any value works; this one keeps the
/// suite quick.
const PATIENCE: Duration = Duration::from_millis(1500);

fn quad(n: usize) -> (String,) {
    (format!("http://ex.org/{n}"),)
}

fn insert(store: &Store, n: usize) {
    let s = quad(n).0;
    store
        .insert(QuadRef::new(
            NamedNodeRef::new(&s).unwrap(),
            NamedNodeRef::new("http://ex.org/p").unwrap(),
            NamedNodeRef::new("http://ex.org/o").unwrap(),
            GraphNameRef::DefaultGraph,
        ))
        .unwrap();
}

fn seed(store: &Store, n: usize) {
    for i in 0..n {
        insert(store, i);
    }
}

/// Spawn a writer, and report whether it finished while the caller still holds
/// whatever it is holding. Never joins on a thread that may be blocked, because
/// a test that hangs reports nothing at all; this one ran for 60 seconds and
/// told us only that it was unhappy.
fn writer_finishes_within(store: &Arc<Store>, first: usize, patience: Duration) -> bool {
    let done = Arc::new(AtomicBool::new(false));
    let (w, d) = (Arc::clone(store), Arc::clone(&done));
    std::thread::spawn(move || {
        for i in first..first + 20 {
            insert(&w, i);
        }
        d.store(true, Ordering::SeqCst);
    });
    let deadline = Instant::now() + patience;
    while Instant::now() < deadline {
        if done.load(Ordering::SeqCst) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    done.load(Ordering::SeqCst)
}

// ── The in-memory store, which is what GraphStore::new builds ───────────

#[test]
fn in_memory_an_open_read_transaction_blocks_every_writer() {
    let store = Arc::new(Store::new().unwrap());
    seed(&store, 100);

    let txn = store.start_transaction().unwrap();
    let before = txn.quads_for_pattern(None, None, None, None).count();
    assert_eq!(before, 100, "the seed did not land");

    let finished = writer_finishes_within(&store, 1000, PATIENCE);
    assert!(
        !finished,
        "a writer completed while a read transaction was open on the IN-MEMORY store. That \
         would be good news, and it would also mean this file's measurement of the cost of \
         holding a transaction is stale and decision 0010 should be re-argued."
    );

    // The reader's view is stable, which it trivially is when nobody can write.
    let after = txn.quads_for_pattern(None, None, None, None).count();
    assert_eq!(after, before, "the view moved under a reader that holds the only lock");

    // Releasing the transaction releases the writer.
    txn.commit().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while store.len().unwrap() < 120 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        store.len().unwrap(),
        120,
        "the writer did not proceed after the transaction committed, so it was not merely \
         blocked on the reader and this is a deadlock rather than a lock"
    );
}

// ── The persistent store, which behaves differently ─────────────────────

#[test]
fn on_disk_a_writer_proceeds_and_the_reader_still_sees_a_stable_view() {
    let dir = std::env::temp_dir().join(format!("oo-txn-disk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = Arc::new(Store::open(&dir).unwrap());
    seed(&store, 100);

    let txn = store.start_transaction().unwrap();
    let before = txn.quads_for_pattern(None, None, None, None).count();

    let finished = writer_finishes_within(&store, 1000, Duration::from_secs(10));
    let after = txn.quads_for_pattern(None, None, None, None).count();
    txn.commit().unwrap();
    let total = store.len().unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        finished,
        "the on-disk store blocked a writer the way the in-memory one does. The two would then \
         behave alike after all, and the note in docs/decisions/0010 about them differing is \
         wrong."
    );
    assert_eq!(
        after, before,
        "the on-disk reader saw a concurrent commit mid-transaction, so repeatable read does \
         not hold and a transaction is NOT a snapshot here"
    );
    assert_eq!(total, 120, "the writer's work should be visible after the reader commits");
}

/// The two backends differ, and that is the finding. Stated as its own test so
/// that it fails loudly if a future Oxigraph makes them agree, in either
/// direction, rather than quietly invalidating the reasoning in 0010.
#[test]
fn the_two_backends_do_not_behave_alike() {
    let mem = Arc::new(Store::new().unwrap());
    seed(&mem, 10);
    let t = mem.start_transaction().unwrap();
    let mem_blocks = !writer_finishes_within(&mem, 500, PATIENCE);
    t.commit().unwrap();

    let dir = std::env::temp_dir().join(format!("oo-txn-cmp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let disk = Arc::new(Store::open(&dir).unwrap());
    seed(&disk, 10);
    let t2 = disk.start_transaction().unwrap();
    let disk_blocks = !writer_finishes_within(&disk, 500, Duration::from_secs(10));
    t2.commit().unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        mem_blocks && !disk_blocks,
        "measured: in-memory blocks writers = {mem_blocks}, on-disk blocks writers = \
         {disk_blocks}. docs/decisions/0010 records that these differ; if they no longer do, \
         the decision's cost argument needs rewriting rather than this assertion relaxing."
    );
}

// ── Phantoms, settled ───────────────────────────────────────────────────

/// Repeatable read permits phantoms in general: a second evaluation of the same
/// PATTERN may return rows a concurrent transaction inserted, even though rows
/// already read do not change. Whether it permits them HERE is the question
/// decision 0010 turns on, and it is answerable by experiment rather than by
/// reading the isolation level's name.
///
/// The probe is deliberately hostile. It repeats the same pattern scan twenty
/// times while a writer commits between every pair of scans, and it scans a
/// pattern the writer is inserting INTO, so a phantom has every chance to
/// appear. It runs on the on-disk store, because that is the one where a writer
/// can commit at all while a reader is open; in memory the writer is blocked,
/// so phantoms are unreachable there for a reason that has nothing to do with
/// the isolation level.
#[test]
fn on_disk_repeated_scans_of_a_growing_pattern_never_return_a_phantom() {
    let dir = std::env::temp_dir().join(format!("oo-txn-phantom-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = Arc::new(Store::open(&dir).unwrap());
    seed(&store, 50);

    let p = NamedNodeRef::new("http://ex.org/p").unwrap();
    let txn = store.start_transaction().unwrap();

    let first = txn.quads_for_pattern(None, Some(p), None, None).count();
    let mut counts = vec![first];

    let stop = Arc::new(AtomicBool::new(false));
    let (w, sflag) = (Arc::clone(&store), Arc::clone(&stop));
    let writer = std::thread::spawn(move || {
        let mut i = 10_000;
        while !sflag.load(Ordering::SeqCst) {
            insert(&w, i);
            i += 1;
            std::thread::sleep(Duration::from_millis(2));
        }
        i - 10_000
    });

    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(5));
        counts.push(txn.quads_for_pattern(None, Some(p), None, None).count());
    }
    stop.store(true, Ordering::SeqCst);
    let written = writer.join().expect("writer thread");
    txn.commit().unwrap();

    let total = store.len().unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        written > 0,
        "the writer committed nothing during the scans, so this probe had no chance to see a \
         phantom and proves nothing. It is the gate on the gate."
    );
    let moved: Vec<_> = counts.iter().filter(|&&c| c != first).collect();
    assert!(
        moved.is_empty(),
        "a repeated scan of the same pattern returned {moved:?} where the first returned \
         {first}, with {written} rows committed underneath it. Repeatable read admits phantoms \
         on this store, a transaction is therefore NOT a snapshot, and the copy in decision \
         0010 is load-bearing for correctness rather than for availability."
    );
    assert_eq!(
        total,
        50 + written,
        "every committed row should be visible to a read started after the transaction closed"
    );
}
