#![cfg(feature = "duckdb")]
//! Integration tests for the DuckDB SQL backbone.
//!
//! These tests cover the two integration points the data pipeline exposes for
//! DuckDB: schema introspection (`onto_import_schema` / `import-schema`) and
//! row extraction via SQL query (`onto_sql_ingest` / `sql-ingest`).

use open_ontologies::schema::SchemaIntrospector;
use open_ontologies::sqlsource::{self, SqlDriver};
use std::collections::HashMap;

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

fn seed_inmemory_db() -> duckdb::Connection {
    // Note: each Connection::open_in_memory() call creates a *separate*
    // in-memory database, so the helper used by tests creates a file-backed
    // DuckDB in a temp dir so the introspector connecting separately can see
    // the same data.
    duckdb::Connection::open_in_memory().unwrap()
}

#[test]
fn detect_driver_classifies_duckdb_strings() {
    assert_eq!(sqlsource::detect_driver("duckdb:///tmp/x.db").unwrap(), SqlDriver::DuckDb);
    assert_eq!(sqlsource::detect_driver(":memory:").unwrap(), SqlDriver::DuckDb);
    assert_eq!(sqlsource::detect_driver("/data/foo.duckdb").unwrap(), SqlDriver::DuckDb);
}

#[test]
fn introspect_duckdb_extracts_tables_and_constraints() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("schema_test.duckdb");
    let path_str = path.to_string_lossy().to_string();

    {
        let conn = duckdb::Connection::open(&path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE customer (
                id INTEGER PRIMARY KEY,
                name VARCHAR NOT NULL,
                email VARCHAR
            );
            CREATE TABLE orders (
                id INTEGER PRIMARY KEY,
                customer_id INTEGER NOT NULL REFERENCES customer(id),
                total DECIMAL(10,2) NOT NULL
            );
            "#,
        )
        .unwrap();
    }

    let tables = SchemaIntrospector::introspect_duckdb(&path_str).unwrap();
    assert_eq!(tables.len(), 2);

    let customer = tables.iter().find(|t| t.name == "customer").unwrap();
    let id_col = customer.columns.iter().find(|c| c.name == "id").unwrap();
    assert!(id_col.is_primary_key, "id should be PK");
    assert!(!id_col.is_nullable, "PK column should be NOT NULL");

    let orders = tables.iter().find(|t| t.name == "orders").unwrap();
    let fk = orders
        .foreign_keys
        .iter()
        .find(|fk| fk.column == "customer_id")
        .expect("orders.customer_id FK should be detected");
    assert_eq!(fk.parent_table, "customer");
    assert_eq!(fk.parent_column, "id");

    // Generated Turtle should reflect the FK as an ObjectProperty pointing to
    // the parent class.
    let turtle = SchemaIntrospector::generate_turtle(&tables, "http://example.org/db/");
    assert!(turtle.contains("db:Customer a owl:Class"));
    assert!(turtle.contains("db:Orders a owl:Class"));
    assert!(turtle.contains("db:orders_customer_id a owl:ObjectProperty"));
    assert!(turtle.contains("rdfs:range db:Customer"));
}

#[test]
fn query_rows_returns_tabular_data_from_duckdb() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("query_test.duckdb");
    let path_str = path.to_string_lossy().to_string();

    {
        let conn = duckdb::Connection::open(&path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE pizza (name VARCHAR, base VARCHAR, price DECIMAL(5,2));
            INSERT INTO pizza VALUES
                ('Margherita', 'Thin', 9.50),
                ('Pepperoni',  'Thin', 11.00);
            "#,
        )
        .unwrap();
    }

    let rows = rt()
        .block_on(sqlsource::query_rows(
            &format!("duckdb://{}", path_str),
            "SELECT name, base, price FROM pizza ORDER BY name",
        ))
        .expect("query_rows succeeds");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get("name").map(String::as_str), Some("Margherita"));
    assert_eq!(rows[0].get("base").map(String::as_str), Some("Thin"));
    assert!(rows[0].get("price").map(String::as_str).unwrap().starts_with("9.5"));
    assert_eq!(rows[1].get("name").map(String::as_str), Some("Pepperoni"));
}

#[test]
fn query_rows_handles_in_memory_with_seed_query() {
    // Validate that an in-memory DuckDB session works end-to-end: SELECT a
    // hand-crafted constant table. This is the lightweight smoke test that
    // does not require any temp file.
    let _ = seed_inmemory_db();
    let rows = rt()
        .block_on(sqlsource::query_rows(
            ":memory:",
            "SELECT * FROM (VALUES (1, 'a'), (2, 'b')) t(id, label)",
        ))
        .expect("in-memory query succeeds");
    assert_eq!(rows.len(), 2);
    let mut sorted: Vec<&HashMap<String, String>> = rows.iter().collect();
    sorted.sort_by_key(|r| r.get("id").cloned().unwrap_or_default());
    assert_eq!(sorted[0].get("id").map(String::as_str), Some("1"));
    assert_eq!(sorted[0].get("label").map(String::as_str), Some("a"));
}
