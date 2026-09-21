//! Shared helper for tests that need a fixture the repository does not carry.
//!
//! Several tests used to `return` when their fixture was missing, three of them
//! without printing anything. A test that returns early reports `ok`, so in CI,
//! where neither `data/crosswalks.parquet` nor the embedding model exists, they
//! counted towards a green run while executing nothing. Reporting success for
//! work that did not happen is the one failure this project is built to catch,
//! and it was in its own suite.
//!
//! `skip_unless` makes the skip loud and, when `OO_REQUIRE_FIXTURES=1` is set,
//! fatal. Set that variable in any job that is supposed to provide fixtures and
//! the suite can no longer quietly shrink.

/// Returns true when the caller should skip. Panics instead if the environment
/// says fixtures are mandatory.
#[allow(dead_code)]
pub fn skip_unless(available: bool, what: &str, how_to_get_it: &str) -> bool {
    if available {
        return false;
    }
    let msg = format!("missing fixture: {what}. {how_to_get_it}");
    if std::env::var("OO_REQUIRE_FIXTURES").as_deref() == Ok("1") {
        panic!("{msg} (OO_REQUIRE_FIXTURES=1, so this is a failure, not a skip)");
    }
    // Distinctive marker so a CI step can count skips rather than let them
    // hide inside a green run.
    eprintln!("SKIPPED_FIXTURE: {msg}");
    true
}

/// Writing a file and forking a process must not overlap in one test binary.
///
/// Confirmed from a CI log rather than guessed at. On 21 September 2026 a lean
/// job printed
///
/// ```text
/// runs: Os { code: 26, kind: ExecutableFileBusy, message: "Text file busy" }
/// ```
///
/// A thread writing a script holds a write file descriptor to it. If another
/// thread forks in that instant, the child inherits that descriptor. The writer
/// closes its own copy and execs the script, but the child still holds one
/// until it reaches its own exec, and Linux refuses to exec a file any process
/// has open for writing. The spawn returns `ExecutableFileBusy`.
///
/// Giving each script its own path does NOT fix it, and that was tried: the
/// race is between ANY write and ANY fork, not between two writers of one
/// file. It never reproduces on macOS, which does not raise this error here,
/// so it looks like a CI defect and is not one.
///
/// Only threads of one process matter, because a descriptor is inherited by a
/// fork and is not shared between unrelated processes, so a mutex per test
/// binary is the whole fix. It lives here because the same defect has now
/// appeared in two test files and the next one should reach for this instead
/// of rediscovering it.
///
/// Hold it across the write AND across the spawn. Both are short.
///
/// ```ignore
/// let script = { let _gate = common::exec_gate(); write_script() };
/// let out = { let _gate = common::exec_gate(); Command::new(&script).output() };
/// ```
///
/// `unwrap_or_else(|e| e.into_inner())`: a test that panics holding this lock
/// poisons it, and the next test would then fail for a reason unrelated to
/// what it checks.
#[allow(dead_code)]
pub fn exec_gate() -> std::sync::MutexGuard<'static, ()> {
    static GATE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    GATE.lock().unwrap_or_else(|e| e.into_inner())
}
