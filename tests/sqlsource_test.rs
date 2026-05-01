//! Tests for `sqlsource::detect_driver` and `duckdb_target` that work even
//! without any SQL feature flag enabled. These exercise the connection-string
//! parsing logic that the data-pipeline docs rely on.

use open_ontologies::sqlsource::{detect_driver, duckdb_target, SqlDriver};

#[test]
fn detects_postgres() {
    assert_eq!(detect_driver("postgres://u:p@h/db").unwrap(), SqlDriver::Postgres);
    assert_eq!(detect_driver("postgresql://u@h:5432/db").unwrap(), SqlDriver::Postgres);
}

#[test]
fn detects_duckdb_variants() {
    assert_eq!(detect_driver("duckdb:///tmp/x.duckdb").unwrap(), SqlDriver::DuckDb);
    assert_eq!(detect_driver("duckdb:/tmp/x.duckdb").unwrap(), SqlDriver::DuckDb);
    assert_eq!(detect_driver(":memory:").unwrap(), SqlDriver::DuckDb);
    assert_eq!(detect_driver("/var/data/warehouse.duckdb").unwrap(), SqlDriver::DuckDb);
    assert_eq!(detect_driver("./local.ddb").unwrap(), SqlDriver::DuckDb);
}

#[test]
fn rejects_unsupported_drivers() {
    assert!(detect_driver("").is_err());
    assert!(detect_driver("mysql://x").is_err());
    assert!(detect_driver("sqlite:foo.db").is_err());
    assert!(detect_driver("/data/file.csv").is_err());
}

#[test]
fn duckdb_target_normalises_prefixes() {
    assert_eq!(duckdb_target("duckdb:///tmp/x.db"), "/tmp/x.db");
    assert_eq!(duckdb_target("duckdb:/tmp/x.db"), "/tmp/x.db");
    assert_eq!(duckdb_target("duckdb://"), ":memory:");
    assert_eq!(duckdb_target(":memory:"), ":memory:");
    assert_eq!(duckdb_target("/data/file.duckdb"), "/data/file.duckdb");
}
