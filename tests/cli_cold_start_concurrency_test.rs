//! Two subcommands starting at once on a cold data directory must both work.
//!
//! Every subcommand opens `<data_dir>/open-ontologies.db` through
//! `StateDb::open`, which converts the file to WAL. Converting to WAL takes an
//! exclusive lock, and SQLite fails that conversion with SQLITE_BUSY without
//! consulting the busy handler, so the five second timeout rusqlite installs by
//! default never applies to it. On a cold directory every process is trying the
//! conversion at once, and the losers exited 1 with `Error: database is locked`.
//!
//! That is not only a test-suite problem. `ls *.ttl | xargs -P8 open-ontologies
//! lint` is the shape of use this fails on, and it failed roughly 40% of
//! invocations locally. CI run 33334852611 is the same defect arriving from two
//! test binaries that both shell out to the CLI: three of the four tests in
//! cli_exit_codes_test.rs failed across two legs, on different tests each time.

use std::process::Command;

const GOOD: &str = "@prefix : <http://ex.org/> .\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n:C a owl:Class ; rdfs:label \"C\" ; rdfs:comment \"a class\" .\n";

/// Eight processes, one cold data directory, no survivors permitted.
///
/// Repeated because the race is a race: a single round passed about 60% of the
/// time even while the defect was live.
#[test]
fn concurrent_subcommands_share_a_cold_data_directory() {
    for round in 0..3 {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");
        let ttl = dir.path().join("ok.ttl");
        std::fs::write(&ttl, GOOD).unwrap();

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let data_dir = data_dir.clone();
                let ttl = ttl.clone();
                std::thread::spawn(move || {
                    Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
                        .args([
                            "--data-dir",
                            data_dir.to_str().unwrap(),
                            "lint",
                            ttl.to_str().unwrap(),
                        ])
                        .output()
                        .expect("binary should run")
                })
            })
            .collect();

        for (i, h) in handles.into_iter().enumerate() {
            let out = h.join().unwrap();
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            assert!(
                out.status.success(),
                "round {round}, process {i} failed on a cold data directory: {text}"
            );
        }
    }
}
