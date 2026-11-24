// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

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

/// A `serve-http` child a test owns. Dropping it kills and reaps the process.
#[allow(dead_code)]
pub struct HttpServer {
    pub child: std::process::Child,
    pub addr: std::net::SocketAddr,
}

impl Drop for HttpServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Start `cmd`, an `open-ontologies serve-http` command without `--host` or
/// `--port`, on a port the server picks itself, and return once the server
/// has said which port it bound.
///
/// Confirmed from a CI log on 23 November 2026 (main, 1ba7ecf8). Two tests in
/// `api_error_json_test` failed together with `ConnectionRefused` on their
/// first request, right after the readiness check had connected to the same
/// address.
///
/// The tests used to reserve a port by binding a listener to port 0, dropping
/// it, passing the number to `serve-http`, and polling with
/// `TcpStream::connect` until a connection succeeded. A successful connect only
/// shows that *something* is listening there. If a sibling test forks while
/// the reservation listener is open, the child inherits the listening socket
/// and keeps it until its own exec. The poll connects to that copy, reports the
/// server ready, and the first real request is refused: the copy has closed
/// and the server has not bound yet, or could not bind because the copy still
/// held the port. It is the fork-inheritance race behind `exec_gate` above,
/// reached through a socket instead of a file.
///
/// Here no test process holds the port at any point, so no child can inherit
/// it, and the address comes from the server's own startup line rather than
/// from a probe that anything can answer.
#[allow(dead_code)]
pub fn spawn_serve_http(mut cmd: std::process::Command) -> HttpServer {
    use std::io::BufRead;
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::{Duration, Instant};

    const READY: &str = "listening on http://";
    cmd.args(["--host", "127.0.0.1", "--port", "0"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    let stderr = child.stderr.take().unwrap();
    let mut server = HttpServer {
        child,
        addr: std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
    };
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        // Ok(address), or Err(everything printed) if stderr closes first.
        let mut seen = Vec::new();
        let mut tx = Some(tx);
        // Keep draining after the address line: a server whose stderr pipe
        // fills up blocks, and one whose pipe is closed panics on its next
        // log line.
        for line in std::io::BufReader::new(stderr).lines() {
            let Ok(line) = line else { break };
            if let Some(rest) = line.split(READY).nth(1)
                && let Some(tx) = tx.take()
            {
                let _ = tx.send(Ok(rest.trim_end_matches("/mcp").to_string()));
            } else if tx.is_some() {
                seen.push(line);
            }
        }
        if let Some(tx) = tx {
            let _ = tx.send(Err(seen.join("\n")));
        }
    });
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(Ok(addr)) => {
                server.addr = addr.parse().unwrap_or_else(|e| {
                    panic!("serve-http reported an address that does not parse ({e}): {addr}")
                });
                return server;
            }
            Ok(Err(stderr)) => panic!("serve-http exited before it bound:\n{stderr}"),
            Err(RecvTimeoutError::Disconnected) => panic!("serve-http stderr reader died"),
            Err(RecvTimeoutError::Timeout) => {}
        }
        assert!(
            Instant::now() < deadline,
            "serve-http did not report a bound address within 30 s"
        );
    }
}
