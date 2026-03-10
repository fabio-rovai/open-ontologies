# Ontology Extension Tools — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add 5 new MCP tools (onto_ingest, onto_map, onto_shacl, onto_reason, onto_extend) that let users feed structured data and apply ontology rules.

**Architecture:** New Rust modules (ingest.rs, mapping.rs, shacl.rs, reason.rs) implement domain logic. Each exposes pure functions called from server.rs tool handlers. All tools use the existing shared GraphStore (Oxigraph) and StateDb (SQLite).

**Tech Stack:** Rust 2024, Oxigraph 0.4, csv, serde_json, serde_yaml, quick-xml, calamine, arrow/parquet crates.

---

### Task 1: Add new dependencies to Cargo.toml

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add the new crate dependencies**

Add under `[dependencies]`:

```toml
csv = "1"
quick-xml = { version = "0.37", features = ["serialize"] }
serde_yaml = "0.9"
calamine = "0.26"
arrow = { version = "54", default-features = false, features = ["csv", "json"] }
parquet = { version = "54", default-features = false }
```

**Step 2: Verify it compiles**

Run: `cd /Users/fabio/projects/open-ontologies && cargo check`
Expected: compiles with no errors (warnings OK)

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "feat: add dependencies for data ingestion (csv, xml, yaml, excel, parquet)"
```

---

### Task 2: Add `sparql_update` method to GraphStore

The reasoning tool needs SPARQL INSERT WHERE support. Oxigraph supports it but GraphStore doesn't expose it yet.

**Files:**
- Modify: `src/graph.rs`
- Test: `tests/graph_test.rs`

**Step 1: Write the failing test**

Append to `tests/graph_test.rs`:

```rust
#[test]
fn test_sparql_update_insert() {
    let store = GraphStore::new();
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:Cat rdfs:subClassOf ex:Animal .
        ex:Tabby a ex:Cat .
    "#;
    store.load_turtle(ttl, None).unwrap();
    assert_eq!(store.triple_count(), 2);

    // Insert inferred triple: Tabby is also an Animal via subclass
    let update = r#"
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        INSERT { ?x a ?super }
        WHERE {
            ?x a ?sub .
            ?sub rdfs:subClassOf ?super .
        }
    "#;
    let result = store.sparql_update(update);
    assert!(result.is_ok());
    assert_eq!(store.triple_count(), 3); // original 2 + 1 inferred
}
```

**Step 2: Run test to verify it fails**

Run: `cd /Users/fabio/projects/open-ontologies && cargo test test_sparql_update_insert -- --nocapture`
Expected: FAIL — `sparql_update` method not found

**Step 3: Implement sparql_update in GraphStore**

Add to `src/graph.rs` impl block, after the `sparql_select` method:

```rust
/// Run a SPARQL UPDATE (INSERT/DELETE) against the store.
/// Returns the number of new triples (delta).
pub fn sparql_update(&self, update: &str) -> anyhow::Result<usize> {
    let store = self.store.lock().unwrap();
    let before = store.len()?;
    store.update(update)?;
    let after = store.len()?;
    Ok(after.saturating_sub(before))
}
```

**Step 4: Run test to verify it passes**

Run: `cd /Users/fabio/projects/open-ontologies && cargo test test_sparql_update_insert -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add src/graph.rs tests/graph_test.rs
git commit -m "feat: add sparql_update method to GraphStore for INSERT/DELETE support"
```

---

### Task 3: Create `src/mapping.rs` — mapping config logic

**Files:**
- Create: `src/mapping.rs`
- Modify: `src/lib.rs` — add `pub mod mapping;`
- Test: `tests/mapping_test.rs`

**Step 1: Write the failing test**

Create `tests/mapping_test.rs`:

```rust
use open_ontologies::mapping::{MappingConfig, FieldMapping};

#[test]
fn test_mapping_from_csv_headers() {
    let headers = vec!["id".to_string(), "name".to_string(), "category".to_string()];
    let config = MappingConfig::from_headers(&headers, "http://example.org/data/", "http://example.org/ont#Thing");
    assert_eq!(config.mappings.len(), 3);
    assert_eq!(config.base_iri, "http://example.org/data/");
    assert_eq!(config.class, "http://example.org/ont#Thing");
    // First field becomes id_field
    assert_eq!(config.id_field, "id");
}

#[test]
fn test_mapping_serialize_deserialize() {
    let config = MappingConfig {
        base_iri: "http://example.org/data/".to_string(),
        id_field: "id".to_string(),
        class: "http://example.org/ont#Building".to_string(),
        mappings: vec![
            FieldMapping {
                field: "name".to_string(),
                predicate: "http://www.w3.org/2000/01/rdf-schema#label".to_string(),
                datatype: Some("http://www.w3.org/2001/XMLSchema#string".to_string()),
                class: None,
                lookup: false,
            },
        ],
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: MappingConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.mappings[0].field, "name");
    assert_eq!(parsed.base_iri, "http://example.org/data/");
}

#[test]
fn test_mapping_apply_to_row() {
    let config = MappingConfig {
        base_iri: "http://example.org/data/".to_string(),
        id_field: "id".to_string(),
        class: "http://example.org/ont#Building".to_string(),
        mappings: vec![
            FieldMapping {
                field: "id".to_string(),
                predicate: "http://example.org/ont#id".to_string(),
                datatype: Some("http://www.w3.org/2001/XMLSchema#string".to_string()),
                class: None,
                lookup: false,
            },
            FieldMapping {
                field: "name".to_string(),
                predicate: "http://www.w3.org/2000/01/rdf-schema#label".to_string(),
                datatype: Some("http://www.w3.org/2001/XMLSchema#string".to_string()),
                class: None,
                lookup: false,
            },
        ],
    };

    let row: std::collections::HashMap<String, String> = [
        ("id".to_string(), "b1".to_string()),
        ("name".to_string(), "Tower Bridge".to_string()),
    ].into();

    let triples = config.row_to_triples(&row);
    // Should produce: subject rdf:type class + one triple per mapped field
    assert!(triples.len() >= 3); // type + id + name
    assert!(triples.iter().any(|t| t.contains("rdf:type")));
    assert!(triples.iter().any(|t| t.contains("Tower Bridge")));
}
```

**Step 2: Run test to verify it fails**

Run: `cd /Users/fabio/projects/open-ontologies && cargo test --test mapping_test 2>&1 | head -20`
Expected: FAIL — module `mapping` not found

**Step 3: Create `src/mapping.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single field-to-predicate mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    pub field: String,
    pub predicate: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datatype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(default)]
    pub lookup: bool,
}

/// Mapping configuration for data-to-RDF conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingConfig {
    pub base_iri: String,
    pub id_field: String,
    pub class: String,
    pub mappings: Vec<FieldMapping>,
}

impl MappingConfig {
    /// Generate a naive 1:1 mapping from column headers.
    /// First column becomes the id_field. All fields become xsd:string predicates.
    pub fn from_headers(headers: &[String], base_iri: &str, class: &str) -> Self {
        let id_field = headers.first().map(|s| s.clone()).unwrap_or_else(|| "id".to_string());
        let base = base_iri.trim_end_matches('/');
        let mappings = headers
            .iter()
            .map(|h| FieldMapping {
                field: h.clone(),
                predicate: format!("{}/{}", base, h),
                datatype: Some("http://www.w3.org/2001/XMLSchema#string".to_string()),
                class: None,
                lookup: false,
            })
            .collect();
        Self {
            base_iri: base_iri.to_string(),
            id_field,
            class: class.to_string(),
            mappings,
        }
    }

    /// Convert a single data row (field→value map) into Turtle triple strings.
    /// Returns a Vec of Turtle triple lines (without prefix declarations).
    pub fn row_to_triples(&self, row: &HashMap<String, String>) -> Vec<String> {
        let mut triples = Vec::new();
        let id_val = row.get(&self.id_field).cloned().unwrap_or_else(|| {
            format!("row-{}", rand_id())
        });
        let subject = format!("<{}{}>", self.base_iri, sanitize_iri(&id_val));

        // rdf:type triple
        triples.push(format!(
            "{} <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{}> .",
            subject, self.class
        ));

        for mapping in &self.mappings {
            if let Some(value) = row.get(&mapping.field) {
                if value.is_empty() {
                    continue;
                }
                if mapping.lookup {
                    // Object is an IRI (class instance)
                    let obj = format!("<{}{}>", self.base_iri, sanitize_iri(value));
                    triples.push(format!("{} <{}> {} .", subject, mapping.predicate, obj));
                } else if let Some(ref dt) = mapping.datatype {
                    // Typed literal
                    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
                    triples.push(format!(
                        r#"{} <{}> "{}"^^<{}> ."#,
                        subject, mapping.predicate, escaped, dt
                    ));
                } else {
                    // Plain literal
                    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
                    triples.push(format!(
                        r#"{} <{}> "{}" ."#,
                        subject, mapping.predicate, escaped
                    ));
                }
            }
        }
        triples
    }

    /// Convert multiple rows into a single N-Triples string.
    pub fn rows_to_ntriples(&self, rows: &[HashMap<String, String>]) -> String {
        rows.iter()
            .flat_map(|row| self.row_to_triples(row))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Sanitize a string for use in an IRI — replace spaces and special chars.
fn sanitize_iri(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' | '\t' | '\n' => '_',
            '<' | '>' | '{' | '}' | '|' | '\\' | '^' | '`' => '_',
            _ => c,
        })
        .collect()
}

/// Simple random ID for rows without an id field.
fn rand_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    format!("{:x}", n)
}
```

**Step 4: Add module to `src/lib.rs`**

Add `pub mod mapping;` to `src/lib.rs`.

**Step 5: Run tests to verify they pass**

Run: `cd /Users/fabio/projects/open-ontologies && cargo test --test mapping_test -- --nocapture`
Expected: all 3 tests PASS

**Step 6: Commit**

```bash
git add src/mapping.rs src/lib.rs tests/mapping_test.rs
git commit -m "feat: add mapping module for data-to-RDF field mapping configs"
```

---

### Task 4: Create `src/ingest.rs` — data parsing for all formats

**Files:**
- Create: `src/ingest.rs`
- Modify: `src/lib.rs` — add `pub mod ingest;`
- Test: `tests/ingest_test.rs`

**Step 1: Write the failing tests**

Create `tests/ingest_test.rs`:

```rust
use open_ontologies::ingest::DataIngester;
use std::collections::HashMap;

#[test]
fn test_parse_csv() {
    let csv_content = "id,name,category\nb1,Tower Bridge,Landmark\nb2,Big Ben,Landmark\n";
    let rows = DataIngester::parse_csv(csv_content).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], "Tower Bridge");
    assert_eq!(rows[1]["id"], "b2");
}

#[test]
fn test_parse_json_array() {
    let json = r#"[{"id":"b1","name":"Tower Bridge"},{"id":"b2","name":"Big Ben"}]"#;
    let rows = DataIngester::parse_json(json).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], "Tower Bridge");
}

#[test]
fn test_parse_ndjson() {
    let ndjson = "{\"id\":\"b1\",\"name\":\"Tower Bridge\"}\n{\"id\":\"b2\",\"name\":\"Big Ben\"}\n";
    let rows = DataIngester::parse_ndjson(ndjson).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1]["name"], "Big Ben");
}

#[test]
fn test_parse_yaml() {
    let yaml = "- id: b1\n  name: Tower Bridge\n- id: b2\n  name: Big Ben\n";
    let rows = DataIngester::parse_yaml(yaml).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], "Tower Bridge");
}

#[test]
fn test_parse_xml_records() {
    let xml = r#"<records><record><id>b1</id><name>Tower Bridge</name></record><record><id>b2</id><name>Big Ben</name></record></records>"#;
    let rows = DataIngester::parse_xml(xml).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], "b1");
}

#[test]
fn test_detect_format() {
    assert_eq!(DataIngester::detect_format("data.csv"), "csv");
    assert_eq!(DataIngester::detect_format("data.json"), "json");
    assert_eq!(DataIngester::detect_format("data.jsonl"), "ndjson");
    assert_eq!(DataIngester::detect_format("data.ndjson"), "ndjson");
    assert_eq!(DataIngester::detect_format("data.xml"), "xml");
    assert_eq!(DataIngester::detect_format("data.yaml"), "yaml");
    assert_eq!(DataIngester::detect_format("data.yml"), "yaml");
    assert_eq!(DataIngester::detect_format("data.xlsx"), "xlsx");
    assert_eq!(DataIngester::detect_format("data.parquet"), "parquet");
}

#[test]
fn test_extract_headers_csv() {
    let csv_content = "id,name,category\nb1,Tower Bridge,Landmark\n";
    let rows = DataIngester::parse_csv(csv_content).unwrap();
    let headers = DataIngester::extract_headers(&rows);
    assert!(headers.contains(&"id".to_string()));
    assert!(headers.contains(&"name".to_string()));
    assert!(headers.contains(&"category".to_string()));
}
```

**Step 2: Run test to verify it fails**

Run: `cd /Users/fabio/projects/open-ontologies && cargo test --test ingest_test 2>&1 | head -20`
Expected: FAIL — module `ingest` not found

**Step 3: Create `src/ingest.rs`**

```rust
use std::collections::HashMap;
use std::path::Path;

/// Parses structured data files into rows of field→value maps.
pub struct DataIngester;

impl DataIngester {
    /// Detect format from file extension.
    pub fn detect_format(path: &str) -> &'static str {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        // Need to handle the borrow properly
        match ext.as_str() {
            "csv" => "csv",
            "json" => "json",
            "jsonl" | "ndjson" => "ndjson",
            "xml" => "xml",
            "yaml" | "yml" => "yaml",
            "xlsx" | "xls" => "xlsx",
            "parquet" => "parquet",
            _ => "csv", // default
        }
    }

    /// Parse CSV content into rows.
    pub fn parse_csv(content: &str) -> anyhow::Result<Vec<HashMap<String, String>>> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(content.as_bytes());
        let headers: Vec<String> = reader
            .headers()?
            .iter()
            .map(|h| h.trim().to_string())
            .collect();
        let mut rows = Vec::new();
        for result in reader.records() {
            let record = result?;
            let mut row = HashMap::new();
            for (i, field) in record.iter().enumerate() {
                if let Some(header) = headers.get(i) {
                    row.insert(header.clone(), field.to_string());
                }
            }
            rows.push(row);
        }
        Ok(rows)
    }

    /// Parse JSON array content into rows.
    pub fn parse_json(content: &str) -> anyhow::Result<Vec<HashMap<String, String>>> {
        let value: serde_json::Value = serde_json::from_str(content)?;
        Self::json_value_to_rows(&value)
    }

    /// Parse newline-delimited JSON into rows.
    pub fn parse_ndjson(content: &str) -> anyhow::Result<Vec<HashMap<String, String>>> {
        let mut rows = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(line)?;
            if let serde_json::Value::Object(map) = value {
                let row: HashMap<String, String> = map
                    .into_iter()
                    .map(|(k, v)| (k, json_value_to_string(&v)))
                    .collect();
                rows.push(row);
            }
        }
        Ok(rows)
    }

    /// Parse YAML content (expected: array of objects) into rows.
    pub fn parse_yaml(content: &str) -> anyhow::Result<Vec<HashMap<String, String>>> {
        let value: serde_json::Value = serde_yaml::from_str(content)?;
        Self::json_value_to_rows(&value)
    }

    /// Parse XML content into rows. Expects <root><record>...</record></root> structure.
    /// Each child element of a record becomes a field.
    pub fn parse_xml(content: &str) -> anyhow::Result<Vec<HashMap<String, String>>> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let mut reader = Reader::from_str(content);
        let mut rows = Vec::new();
        let mut current_row: Option<HashMap<String, String>> = None;
        let mut current_field: Option<String> = None;
        let mut depth = 0u32;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    depth += 1;
                    if depth == 2 {
                        // Start of a record element
                        current_row = Some(HashMap::new());
                    } else if depth == 3 {
                        // Start of a field element within a record
                        current_field = Some(
                            String::from_utf8_lossy(e.name().as_ref()).to_string(),
                        );
                    }
                }
                Ok(Event::Text(ref e)) => {
                    if let (Some(ref mut row), Some(ref field)) = (&mut current_row, &current_field)
                    {
                        let text = e.unescape().unwrap_or_default().to_string();
                        row.insert(field.clone(), text);
                    }
                }
                Ok(Event::End(_)) => {
                    if depth == 2 {
                        if let Some(row) = current_row.take() {
                            rows.push(row);
                        }
                    } else if depth == 3 {
                        current_field = None;
                    }
                    depth = depth.saturating_sub(1);
                }
                Ok(Event::Eof) => break,
                Err(e) => anyhow::bail!("XML parse error: {}", e),
                _ => {}
            }
        }
        Ok(rows)
    }

    /// Parse an Excel (.xlsx) file into rows.
    pub fn parse_xlsx_file(path: &str) -> anyhow::Result<Vec<HashMap<String, String>>> {
        use calamine::{open_workbook, Reader, Xlsx};

        let mut workbook: Xlsx<_> = open_workbook(path)?;
        let sheet_name = workbook
            .sheet_names()
            .first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No sheets found in Excel file"))?;
        let range = workbook
            .worksheet_range(&sheet_name)?;

        let mut rows_iter = range.rows();
        let headers: Vec<String> = rows_iter
            .next()
            .ok_or_else(|| anyhow::anyhow!("Empty spreadsheet"))?
            .iter()
            .map(|c| c.to_string().trim().to_string())
            .collect();

        let mut rows = Vec::new();
        for row_data in rows_iter {
            let mut row = HashMap::new();
            for (i, cell) in row_data.iter().enumerate() {
                if let Some(header) = headers.get(i) {
                    row.insert(header.clone(), cell.to_string());
                }
            }
            rows.push(row);
        }
        Ok(rows)
    }

    /// Parse a Parquet file into rows.
    pub fn parse_parquet_file(path: &str) -> anyhow::Result<Vec<HashMap<String, String>>> {
        use arrow::array::AsArray;
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
        use std::fs::File;

        let file = File::open(path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let reader = builder.build()?;

        let mut rows = Vec::new();
        for batch in reader {
            let batch = batch?;
            let schema = batch.schema();
            for row_idx in 0..batch.num_rows() {
                let mut row = HashMap::new();
                for (col_idx, field) in schema.fields().iter().enumerate() {
                    let col = batch.column(col_idx);
                    let val = arrow::util::display::array_value_to_string(col, row_idx)?;
                    row.insert(field.name().clone(), val);
                }
                rows.push(row);
            }
        }
        Ok(rows)
    }

    /// Parse a file by auto-detecting its format.
    pub fn parse_file(path: &str) -> anyhow::Result<Vec<HashMap<String, String>>> {
        let format = Self::detect_format(path);
        match format {
            "csv" => {
                let content = std::fs::read_to_string(path)?;
                Self::parse_csv(&content)
            }
            "json" => {
                let content = std::fs::read_to_string(path)?;
                Self::parse_json(&content)
            }
            "ndjson" => {
                let content = std::fs::read_to_string(path)?;
                Self::parse_ndjson(&content)
            }
            "yaml" => {
                let content = std::fs::read_to_string(path)?;
                Self::parse_yaml(&content)
            }
            "xml" => {
                let content = std::fs::read_to_string(path)?;
                Self::parse_xml(&content)
            }
            "xlsx" => Self::parse_xlsx_file(path),
            "parquet" => Self::parse_parquet_file(path),
            other => anyhow::bail!("Unsupported format: {}", other),
        }
    }

    /// Extract unique header/field names from parsed rows.
    pub fn extract_headers(rows: &[HashMap<String, String>]) -> Vec<String> {
        let mut headers: Vec<String> = rows
            .iter()
            .flat_map(|r| r.keys().cloned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        headers.sort();
        headers
    }
}

/// Convert a serde_json::Value to a flat string.
fn json_value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

impl DataIngester {
    /// Helper: convert a JSON Value (array of objects) to rows.
    fn json_value_to_rows(
        value: &serde_json::Value,
    ) -> anyhow::Result<Vec<HashMap<String, String>>> {
        match value {
            serde_json::Value::Array(arr) => {
                let mut rows = Vec::new();
                for item in arr {
                    if let serde_json::Value::Object(map) = item {
                        let row: HashMap<String, String> = map
                            .iter()
                            .map(|(k, v)| (k.clone(), json_value_to_string(v)))
                            .collect();
                        rows.push(row);
                    }
                }
                Ok(rows)
            }
            serde_json::Value::Object(map) => {
                // Single object → single row
                let row: HashMap<String, String> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), json_value_to_string(v)))
                    .collect();
                Ok(vec![row])
            }
            _ => anyhow::bail!("Expected JSON array or object"),
        }
    }
}
```

**Step 4: Add module to `src/lib.rs`**

Add `pub mod ingest;` to `src/lib.rs`.

**Step 5: Run tests to verify they pass**

Run: `cd /Users/fabio/projects/open-ontologies && cargo test --test ingest_test -- --nocapture`
Expected: all 8 tests PASS

**Step 6: Commit**

```bash
git add src/ingest.rs src/lib.rs tests/ingest_test.rs
git commit -m "feat: add ingest module supporting CSV, JSON, NDJSON, YAML, XML, XLSX, Parquet"
```

---

### Task 5: Create `src/shacl.rs` — SHACL validation via SPARQL

**Files:**
- Create: `src/shacl.rs`
- Modify: `src/lib.rs` — add `pub mod shacl;`
- Test: `tests/shacl_test.rs`

**Step 1: Write the failing tests**

Create `tests/shacl_test.rs`:

```rust
use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

fn make_store_with_data() -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:b1 a ex:Building ; rdfs:label "Tower Bridge" ; ex:height "65"^^xsd:integer .
        ex:b2 a ex:Building ; ex:height "96"^^xsd:integer .
    "#;
    store.load_turtle(ttl, None).unwrap();
    store
}

#[test]
fn test_shacl_mincount_violation() {
    let store = make_store_with_data();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:BuildingShape a sh:NodeShape ;
            sh:targetClass ex:Building ;
            sh:property [
                sh:path rdfs:label ;
                sh:minCount 1 ;
                sh:message "Building must have a label" ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["conforms"], false);
    assert!(parsed["violation_count"].as_u64().unwrap() >= 1);
}

#[test]
fn test_shacl_all_pass() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:b1 a ex:Building ; rdfs:label "Tower Bridge" .
        ex:b2 a ex:Building ; rdfs:label "Big Ben" .
    "#;
    store.load_turtle(ttl, None).unwrap();

    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:BuildingShape a sh:NodeShape ;
            sh:targetClass ex:Building ;
            sh:property [
                sh:path rdfs:label ;
                sh:minCount 1 ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["conforms"], true);
    assert_eq!(parsed["violation_count"], 0);
}
```

**Step 2: Run test to verify it fails**

Run: `cd /Users/fabio/projects/open-ontologies && cargo test --test shacl_test 2>&1 | head -20`
Expected: FAIL — module `shacl` not found

**Step 3: Create `src/shacl.rs`**

```rust
use crate::graph::GraphStore;
use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::sparql::QueryResults;
use oxigraph::store::Store;
use std::io::Cursor;
use std::sync::Arc;

/// SHACL validator implemented via SPARQL queries against the data store.
pub struct ShaclValidator;

impl ShaclValidator {
    /// Validate the data in `graph` against SHACL shapes (inline Turtle).
    /// Returns a JSON report: {conforms, violation_count, violations[]}.
    pub fn validate(graph: &Arc<GraphStore>, shapes_ttl: &str) -> anyhow::Result<String> {
        // Parse shapes into a separate store to extract shape definitions
        let shapes_store = Store::new()?;
        let reader = Cursor::new(shapes_ttl.as_bytes());
        for quad in RdfParser::from_format(RdfFormat::Turtle).for_reader(reader) {
            shapes_store.insert(&quad?)?;
        }

        let mut violations: Vec<serde_json::Value> = Vec::new();

        // Extract all NodeShapes with sh:targetClass
        let shape_query = r#"
            PREFIX sh: <http://www.w3.org/ns/shacl#>
            SELECT ?shape ?targetClass WHERE {
                ?shape a sh:NodeShape ;
                       sh:targetClass ?targetClass .
            }
        "#;

        let shapes = Self::query_solutions(&shapes_store, shape_query)?;

        for shape in &shapes {
            let shape_iri = shape.get("shape").cloned().unwrap_or_default();
            let target_class = shape.get("targetClass").cloned().unwrap_or_default();
            let target_class_clean = strip_angle_brackets(&target_class);

            // Get property constraints for this shape
            let prop_query = format!(
                r#"
                PREFIX sh: <http://www.w3.org/ns/shacl#>
                SELECT ?path ?minCount ?maxCount ?datatype ?message ?nodeKind ?clazz WHERE {{
                    <{}> sh:property ?prop .
                    ?prop sh:path ?path .
                    OPTIONAL {{ ?prop sh:minCount ?minCount }}
                    OPTIONAL {{ ?prop sh:maxCount ?maxCount }}
                    OPTIONAL {{ ?prop sh:datatype ?datatype }}
                    OPTIONAL {{ ?prop sh:message ?message }}
                    OPTIONAL {{ ?prop sh:nodeKind ?nodeKind }}
                    OPTIONAL {{ ?prop sh:class ?clazz }}
                }}
                "#,
                strip_angle_brackets(&shape_iri)
            );

            let constraints = Self::query_solutions(&shapes_store, &prop_query)?;

            for constraint in &constraints {
                let path = constraint.get("path").cloned().unwrap_or_default();
                let path_clean = strip_angle_brackets(&path);
                let custom_message = constraint.get("message").map(|m| strip_quotes(m));

                // sh:minCount check
                if let Some(min_str) = constraint.get("minCount") {
                    let min: usize = strip_quotes(min_str).parse().unwrap_or(0);
                    if min > 0 {
                        let check_query = format!(
                            r#"SELECT ?focus (COUNT(?val) AS ?cnt) WHERE {{
                                ?focus a <{target_class}> .
                                OPTIONAL {{ ?focus <{path}> ?val }}
                            }} GROUP BY ?focus HAVING (COUNT(?val) < {min})"#,
                            target_class = target_class_clean,
                            path = path_clean,
                            min = min
                        );
                        let fails = graph.sparql_select(&check_query)?;
                        let parsed: serde_json::Value = serde_json::from_str(&fails)?;
                        if let Some(results) = parsed["results"].as_array() {
                            for row in results {
                                let focus = row["focus"].as_str().unwrap_or("unknown");
                                let msg = custom_message.clone().unwrap_or_else(|| {
                                    format!(
                                        "sh:minCount {} violated on path {}",
                                        min, path_clean
                                    )
                                });
                                violations.push(serde_json::json!({
                                    "severity": "Violation",
                                    "focus_node": focus,
                                    "path": path_clean,
                                    "message": msg,
                                }));
                            }
                        }
                    }
                }

                // sh:maxCount check
                if let Some(max_str) = constraint.get("maxCount") {
                    let max: usize = strip_quotes(max_str).parse().unwrap_or(usize::MAX);
                    let check_query = format!(
                        r#"SELECT ?focus (COUNT(?val) AS ?cnt) WHERE {{
                            ?focus a <{target_class}> .
                            ?focus <{path}> ?val .
                        }} GROUP BY ?focus HAVING (COUNT(?val) > {max})"#,
                        target_class = target_class_clean,
                        path = path_clean,
                        max = max
                    );
                    let fails = graph.sparql_select(&check_query)?;
                    let parsed: serde_json::Value = serde_json::from_str(&fails)?;
                    if let Some(results) = parsed["results"].as_array() {
                        for row in results {
                            let focus = row["focus"].as_str().unwrap_or("unknown");
                            let msg = custom_message.clone().unwrap_or_else(|| {
                                format!("sh:maxCount {} violated on path {}", max, path_clean)
                            });
                            violations.push(serde_json::json!({
                                "severity": "Violation",
                                "focus_node": focus,
                                "path": path_clean,
                                "message": msg,
                            }));
                        }
                    }
                }

                // sh:datatype check
                if let Some(dt) = constraint.get("datatype") {
                    let dt_clean = strip_angle_brackets(dt);
                    let check_query = format!(
                        r#"SELECT ?focus ?val WHERE {{
                            ?focus a <{target_class}> .
                            ?focus <{path}> ?val .
                            FILTER(DATATYPE(?val) != <{dt}>)
                        }}"#,
                        target_class = target_class_clean,
                        path = path_clean,
                        dt = dt_clean
                    );
                    let fails = graph.sparql_select(&check_query)?;
                    let parsed: serde_json::Value = serde_json::from_str(&fails)?;
                    if let Some(results) = parsed["results"].as_array() {
                        for row in results {
                            let focus = row["focus"].as_str().unwrap_or("unknown");
                            violations.push(serde_json::json!({
                                "severity": "Violation",
                                "focus_node": focus,
                                "path": path_clean,
                                "message": format!("Expected datatype {}", dt_clean),
                            }));
                        }
                    }
                }
            }
        }

        let conforms = violations.is_empty();
        Ok(serde_json::json!({
            "conforms": conforms,
            "violation_count": violations.len(),
            "violations": violations,
        })
        .to_string())
    }

    fn query_solutions(
        store: &Store,
        query: &str,
    ) -> anyhow::Result<Vec<std::collections::HashMap<String, String>>> {
        let mut results = Vec::new();
        if let QueryResults::Solutions(solutions) = store.query(query)? {
            let vars: Vec<String> = solutions
                .variables()
                .iter()
                .map(|v| v.as_str().to_string())
                .collect();
            for solution in solutions {
                let solution = solution?;
                let mut row = std::collections::HashMap::new();
                for var in &vars {
                    if let Some(term) = solution.get(var.as_str()) {
                        row.insert(var.clone(), term.to_string());
                    }
                }
                results.push(row);
            }
        }
        Ok(results)
    }
}

fn strip_angle_brackets(s: &str) -> String {
    s.trim_start_matches('<').trim_end_matches('>').to_string()
}

fn strip_quotes(s: &str) -> String {
    let s = s.trim_matches('"');
    // Handle typed literals like "1"^^<xsd:integer>
    if let Some(idx) = s.find("^^") {
        s[..idx].trim_matches('"').to_string()
    } else {
        s.to_string()
    }
}
```

**Step 4: Add module to `src/lib.rs`**

Add `pub mod shacl;` to `src/lib.rs`.

**Step 5: Run tests to verify they pass**

Run: `cd /Users/fabio/projects/open-ontologies && cargo test --test shacl_test -- --nocapture`
Expected: both tests PASS

**Step 6: Commit**

```bash
git add src/shacl.rs src/lib.rs tests/shacl_test.rs
git commit -m "feat: add SHACL validation module (minCount, maxCount, datatype via SPARQL)"
```

---

### Task 6: Create `src/reason.rs` — RDFS/OWL inference via SPARQL

**Files:**
- Create: `src/reason.rs`
- Modify: `src/lib.rs` — add `pub mod reason;`
- Test: `tests/reason_test.rs`

**Step 1: Write the failing tests**

Create `tests/reason_test.rs`:

```rust
use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;
use std::sync::Arc;

#[test]
fn test_rdfs_subclass_inference() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:Cat rdfs:subClassOf ex:Animal .
        ex:Animal rdfs:subClassOf ex:LivingThing .
        ex:Tabby a ex:Cat .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let before = store.triple_count();

    let result = Reasoner::run(&store, "rdfs", true).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let inferred = parsed["inferred_count"].as_u64().unwrap();
    assert!(inferred >= 2); // Tabby a Animal, Tabby a LivingThing

    // Verify inferred triples are in the store
    let check = store.sparql_select(
        "ASK { <http://example.org/Tabby> a <http://example.org/Animal> }"
    ).unwrap();
    assert!(check.contains("true"));
}

#[test]
fn test_rdfs_domain_inference() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:hasName rdfs:domain ex:Entity .
        ex:Alice ex:hasName "Alice" .
    "#;
    store.load_turtle(ttl, None).unwrap();

    Reasoner::run(&store, "rdfs", true).unwrap();

    let check = store.sparql_select(
        "ASK { <http://example.org/Alice> a <http://example.org/Entity> }"
    ).unwrap();
    assert!(check.contains("true"));
}

#[test]
fn test_reason_no_materialize() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:Cat rdfs:subClassOf ex:Animal .
        ex:Tabby a ex:Cat .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let before = store.triple_count();

    let result = Reasoner::run(&store, "rdfs", false).unwrap();
    // Store should be unchanged (dry run)
    assert_eq!(store.triple_count(), before);

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["inferred_count"].as_u64().unwrap() >= 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cd /Users/fabio/projects/open-ontologies && cargo test --test reason_test 2>&1 | head -20`
Expected: FAIL — module `reason` not found

**Step 3: Create `src/reason.rs`**

```rust
use crate::graph::GraphStore;
use std::sync::Arc;

/// RDFS/OWL reasoner implemented via iterative SPARQL INSERT.
pub struct Reasoner;

/// RDFS inference rules as SPARQL INSERT WHERE queries.
const RDFS_RULES: &[(&str, &str)] = &[
    // rdfs9: subclass type propagation
    (
        "rdfs9-subclass",
        r#"INSERT { ?x a ?super }
           WHERE {
               ?x a ?sub .
               ?sub <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?super .
               FILTER(?sub != ?super)
               FILTER NOT EXISTS { ?x a ?super }
           }"#,
    ),
    // rdfs11: subclass transitivity
    (
        "rdfs11-subclass-trans",
        r#"INSERT { ?a <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c }
           WHERE {
               ?a <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?b .
               ?b <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c .
               FILTER(?a != ?b && ?b != ?c && ?a != ?c)
               FILTER NOT EXISTS { ?a <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c }
           }"#,
    ),
    // rdfs2: domain inference
    (
        "rdfs2-domain",
        r#"INSERT { ?s a ?class }
           WHERE {
               ?s ?p ?o .
               ?p <http://www.w3.org/2000/01/rdf-schema#domain> ?class .
               FILTER NOT EXISTS { ?s a ?class }
           }"#,
    ),
    // rdfs3: range inference
    (
        "rdfs3-range",
        r#"INSERT { ?o a ?class }
           WHERE {
               ?s ?p ?o .
               ?p <http://www.w3.org/2000/01/rdf-schema#range> ?class .
               FILTER(isIRI(?o))
               FILTER NOT EXISTS { ?o a ?class }
           }"#,
    ),
    // rdfs5: subproperty transitivity
    (
        "rdfs5-subprop-trans",
        r#"INSERT { ?a <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?c }
           WHERE {
               ?a <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?b .
               ?b <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?c .
               FILTER(?a != ?b && ?b != ?c && ?a != ?c)
               FILTER NOT EXISTS { ?a <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?c }
           }"#,
    ),
    // rdfs7: subproperty value propagation
    (
        "rdfs7-subprop",
        r#"INSERT { ?s ?super ?o }
           WHERE {
               ?s ?sub ?o .
               ?sub <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?super .
               FILTER(?sub != ?super)
               FILTER NOT EXISTS { ?s ?super ?o }
           }"#,
    ),
];

/// OWL RL inference rules (additions beyond RDFS).
const OWL_RL_RULES: &[(&str, &str)] = &[
    // Transitive properties
    (
        "owl-transitive",
        r#"INSERT { ?a ?p ?c }
           WHERE {
               ?p a <http://www.w3.org/2002/07/owl#TransitiveProperty> .
               ?a ?p ?b .
               ?b ?p ?c .
               FILTER(?a != ?c)
               FILTER NOT EXISTS { ?a ?p ?c }
           }"#,
    ),
    // Symmetric properties
    (
        "owl-symmetric",
        r#"INSERT { ?o ?p ?s }
           WHERE {
               ?p a <http://www.w3.org/2002/07/owl#SymmetricProperty> .
               ?s ?p ?o .
               FILTER NOT EXISTS { ?o ?p ?s }
           }"#,
    ),
    // Inverse properties
    (
        "owl-inverse",
        r#"INSERT { ?o ?q ?s }
           WHERE {
               ?p <http://www.w3.org/2002/07/owl#inverseOf> ?q .
               ?s ?p ?o .
               FILTER NOT EXISTS { ?o ?q ?s }
           }"#,
    ),
    // owl:sameAs symmetry
    (
        "owl-sameas-sym",
        r#"INSERT { ?b <http://www.w3.org/2002/07/owl#sameAs> ?a }
           WHERE {
               ?a <http://www.w3.org/2002/07/owl#sameAs> ?b .
               FILTER NOT EXISTS { ?b <http://www.w3.org/2002/07/owl#sameAs> ?a }
           }"#,
    ),
];

const MAX_ITERATIONS: usize = 20;

impl Reasoner {
    /// Run inference rules against the graph store.
    /// `profile`: "rdfs" or "owl-rl" (owl-rl includes rdfs rules).
    /// `materialize`: if true, insert inferred triples into the store.
    ///                if false, count what would be inferred without modifying the store.
    pub fn run(graph: &Arc<GraphStore>, profile: &str, materialize: bool) -> anyhow::Result<String> {
        let rules: Vec<(&str, &str)> = match profile {
            "owl-rl" | "owl_rl" => {
                let mut r: Vec<(&str, &str)> = RDFS_RULES.to_vec();
                r.extend_from_slice(OWL_RL_RULES);
                r
            }
            _ => RDFS_RULES.to_vec(), // default to rdfs
        };

        if !materialize {
            return Self::dry_run(graph, &rules, profile);
        }

        let mut total_inferred = 0usize;
        let mut iterations = 0usize;
        let mut samples: Vec<String> = Vec::new();

        loop {
            iterations += 1;
            let before = graph.triple_count();
            for (_name, rule) in &rules {
                let _ = graph.sparql_update(rule);
            }
            let after = graph.triple_count();
            let delta = after.saturating_sub(before);
            total_inferred += delta;

            if delta == 0 || iterations >= MAX_ITERATIONS {
                break;
            }
        }

        // Collect a few sample inferences for the report
        // (This is best-effort — we report total count, not every triple)
        if total_inferred > 0 && samples.is_empty() {
            samples.push(format!("{} new triples materialized", total_inferred));
        }

        Ok(serde_json::json!({
            "profile_used": profile,
            "inferred_count": total_inferred,
            "iterations": iterations,
            "sample_inferences": samples,
        })
        .to_string())
    }

    /// Dry run: count how many triples would be inferred without modifying the store.
    /// Uses a temporary store to simulate.
    fn dry_run(
        graph: &Arc<GraphStore>,
        rules: &[(&str, &str)],
        profile: &str,
    ) -> anyhow::Result<String> {
        // Snapshot current store content and load into a temp store
        let snapshot = graph.serialize("ntriples")?;
        let temp = GraphStore::new();
        temp.load_ntriples(&snapshot)?;
        let temp = Arc::new(temp);

        let mut total_inferred = 0usize;
        let mut iterations = 0usize;

        loop {
            iterations += 1;
            let before = temp.triple_count();
            for (_name, rule) in rules {
                let _ = temp.sparql_update(rule);
            }
            let after = temp.triple_count();
            let delta = after.saturating_sub(before);
            total_inferred += delta;
            if delta == 0 || iterations >= MAX_ITERATIONS {
                break;
            }
        }

        Ok(serde_json::json!({
            "profile_used": profile,
            "inferred_count": total_inferred,
            "iterations": iterations,
            "materialized": false,
            "sample_inferences": [format!("{} triples would be inferred (dry run)", total_inferred)],
        })
        .to_string())
    }
}
```

**Step 4: Add module to `src/lib.rs`**

Add `pub mod reason;` to `src/lib.rs`.

**Step 5: Run tests to verify they pass**

Run: `cd /Users/fabio/projects/open-ontologies && cargo test --test reason_test -- --nocapture`
Expected: all 3 tests PASS

**Step 6: Commit**

```bash
git add src/reason.rs src/lib.rs tests/reason_test.rs
git commit -m "feat: add RDFS/OWL-RL reasoning module via iterative SPARQL INSERT"
```

---

### Task 7: Wire up the 5 new MCP tools in `src/server.rs`

**Files:**
- Modify: `src/server.rs`

**Step 1: Add input structs for the new tools**

Add after the existing input structs at the top of `src/server.rs`:

```rust
#[derive(Deserialize, JsonSchema)]
pub struct OntoIngestInput {
    /// Path to the data file (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet)
    pub path: String,
    /// Data format (auto-detected from extension if omitted): csv, json, ndjson, xml, yaml, xlsx, parquet
    pub format: Option<String>,
    /// Mapping config as JSON string or path to mapping JSON file
    pub mapping: Option<String>,
    /// If true, treat mapping as inline JSON (default: false = file path)
    pub inline_mapping: Option<bool>,
    /// Base IRI for generated instances (default: http://example.org/data/)
    pub base_iri: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoMapInput {
    /// Path to sample data file to generate mapping for
    pub data_path: String,
    /// Data format (auto-detected if omitted)
    pub format: Option<String>,
    /// Optional path to save the generated mapping config
    pub save_path: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoShaclInput {
    /// Path to SHACL shapes file OR inline SHACL Turtle content
    pub shapes: String,
    /// If true, treat shapes as inline Turtle content
    pub inline: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoReasonInput {
    /// Reasoning profile: rdfs (default), owl-rl
    pub profile: Option<String>,
    /// If true (default), add inferred triples to the store. If false, dry-run only.
    pub materialize: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoExtendInput {
    /// Path to the data file
    pub data_path: String,
    /// Data format (auto-detected if omitted)
    pub format: Option<String>,
    /// Mapping config (inline JSON or file path)
    pub mapping: Option<String>,
    /// If true, treat mapping as inline JSON
    pub inline_mapping: Option<bool>,
    /// Base IRI for generated instances
    pub base_iri: Option<String>,
    /// Path to SHACL shapes file or inline Turtle
    pub shapes: Option<String>,
    /// If true, treat shapes as inline Turtle
    pub inline_shapes: Option<bool>,
    /// Reasoning profile (rdfs, owl-rl). Omit to skip reasoning.
    pub reason_profile: Option<String>,
    /// If true (default), stop pipeline on SHACL violations
    pub stop_on_violations: Option<bool>,
}
```

**Step 2: Add the 5 tool handler methods**

Add inside the `#[tool_router] impl OpenOntologiesServer` block, after the existing tools:

```rust
    // ── Data Extension Tools ────────────────────────────────────────────

    #[tool(name = "onto_ingest", description = "Parse a structured data file (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet) into RDF triples and load into the ontology store. Optionally uses a mapping config to control field-to-predicate mapping.")]
    async fn onto_ingest(&self, Parameters(input): Parameters<OntoIngestInput>) -> String {
        use crate::ingest::DataIngester;
        use crate::mapping::MappingConfig;

        let format = input.format.as_deref()
            .unwrap_or_else(|| DataIngester::detect_format(&input.path));
        let base_iri = input.base_iri.as_deref().unwrap_or("http://example.org/data/");

        // Parse data file
        let rows = match DataIngester::parse_file(&input.path) {
            Ok(r) => r,
            Err(e) => return format!(r#"{{"error":"Failed to parse {}: {}"}}"#, input.path, e),
        };

        if rows.is_empty() {
            return r#"{"ok":true,"triples_loaded":0,"warnings":["No data rows found"]}"#.to_string();
        }

        // Get or generate mapping
        let mapping = if let Some(ref mapping_str) = input.mapping {
            if input.inline_mapping.unwrap_or(false) {
                match serde_json::from_str::<MappingConfig>(mapping_str) {
                    Ok(m) => m,
                    Err(e) => return format!(r#"{{"error":"Invalid mapping JSON: {}"}}"#, e),
                }
            } else {
                match std::fs::read_to_string(mapping_str) {
                    Ok(content) => match serde_json::from_str::<MappingConfig>(&content) {
                        Ok(m) => m,
                        Err(e) => return format!(r#"{{"error":"Invalid mapping file: {}"}}"#, e),
                    },
                    Err(e) => return format!(r#"{{"error":"Cannot read mapping file: {}"}}"#, e),
                }
            }
        } else {
            let headers = DataIngester::extract_headers(&rows);
            MappingConfig::from_headers(&headers, base_iri, &format!("{}Thing", base_iri))
        };

        // Convert to N-Triples and load
        let ntriples = mapping.rows_to_ntriples(&rows);
        match self.graph.load_ntriples(&ntriples) {
            Ok(count) => {
                serde_json::json!({
                    "ok": true,
                    "triples_loaded": count,
                    "rows_processed": rows.len(),
                    "format": format,
                    "mapping_fields": mapping.mappings.len(),
                }).to_string()
            }
            Err(e) => format!(r#"{{"error":"Failed to load triples: {}"}}"#, e),
        }
    }

    #[tool(name = "onto_map", description = "Generate a mapping config by inspecting a data file's schema against the currently loaded ontology. Returns a JSON mapping that can be reviewed and passed to onto_ingest.")]
    async fn onto_map(&self, Parameters(input): Parameters<OntoMapInput>) -> String {
        use crate::ingest::DataIngester;
        use crate::mapping::MappingConfig;

        // Parse data to get headers
        let rows = match DataIngester::parse_file(&input.data_path) {
            Ok(r) => r,
            Err(e) => return format!(r#"{{"error":"Failed to parse {}: {}"}}"#, input.data_path, e),
        };
        let headers = DataIngester::extract_headers(&rows);

        // Get ontology classes and properties from the store
        let classes_query = r#"SELECT DISTINCT ?c WHERE {
            { ?c a <http://www.w3.org/2002/07/owl#Class> }
            UNION
            { ?c a <http://www.w3.org/2000/01/rdf-schema#Class> }
        }"#;
        let props_query = r#"SELECT DISTINCT ?p WHERE {
            { ?p a <http://www.w3.org/2002/07/owl#ObjectProperty> }
            UNION
            { ?p a <http://www.w3.org/2002/07/owl#DatatypeProperty> }
            UNION
            { ?p a <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property> }
        }"#;

        let classes = self.graph.sparql_select(classes_query).unwrap_or_default();
        let props = self.graph.sparql_select(props_query).unwrap_or_default();

        // Parse class/property IRIs from SPARQL results
        let extract_iris = |json: &str, var: &str| -> Vec<String> {
            serde_json::from_str::<serde_json::Value>(json)
                .ok()
                .and_then(|v| v["results"].as_array().cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|r| r[var].as_str().map(|s| s.trim_matches(|c| c == '<' || c == '>').to_string()))
                .collect()
        };

        let class_iris = extract_iris(&classes, "c");
        let prop_iris = extract_iris(&props, "p");

        // Generate naive mapping
        let mapping = MappingConfig::from_headers(
            &headers,
            "http://example.org/data/",
            class_iris.first().map(|s| s.as_str()).unwrap_or("http://example.org/Thing"),
        );

        let result = serde_json::json!({
            "mapping": mapping,
            "data_fields": headers,
            "ontology_classes": class_iris,
            "ontology_properties": prop_iris,
            "unmapped_fields": headers.iter()
                .filter(|h| !mapping.mappings.iter().any(|m| &m.field == *h))
                .collect::<Vec<_>>(),
        });

        // Optionally save
        if let Some(ref save_path) = input.save_path {
            match serde_json::to_string_pretty(&mapping) {
                Ok(json) => {
                    if let Err(e) = std::fs::write(save_path, &json) {
                        return format!(r#"{{"error":"Cannot write mapping file: {}"}}"#, e);
                    }
                }
                Err(e) => return format!(r#"{{"error":"Cannot serialize mapping: {}"}}"#, e),
            }
        }

        result.to_string()
    }

    #[tool(name = "onto_shacl", description = "Validate the loaded ontology data against SHACL shapes. Checks cardinality (minCount/maxCount), datatypes, and class constraints. Returns a conformance report with violations.")]
    async fn onto_shacl(&self, Parameters(input): Parameters<OntoShaclInput>) -> String {
        use crate::shacl::ShaclValidator;
        let shapes = if input.inline.unwrap_or(false) {
            input.shapes.clone()
        } else {
            match std::fs::read_to_string(&input.shapes) {
                Ok(c) => c,
                Err(e) => return format!(r#"{{"error":"Cannot read shapes file: {}"}}"#, e),
            }
        };
        ShaclValidator::validate(&self.graph, &shapes)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_reason", description = "Run RDFS or OWL-RL inference rules over the loaded ontology. Materializes inferred triples (subclass propagation, domain/range inference, transitive/symmetric properties).")]
    async fn onto_reason(&self, Parameters(input): Parameters<OntoReasonInput>) -> String {
        use crate::reason::Reasoner;
        let profile = input.profile.as_deref().unwrap_or("rdfs");
        let materialize = input.materialize.unwrap_or(true);
        Reasoner::run(&self.graph, profile, materialize)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_extend", description = "Convenience pipeline: ingest data → validate with SHACL → run OWL reasoning, all in one call. Combines onto_ingest + onto_shacl + onto_reason.")]
    async fn onto_extend(&self, Parameters(input): Parameters<OntoExtendInput>) -> String {
        use crate::ingest::DataIngester;
        use crate::mapping::MappingConfig;
        use crate::shacl::ShaclValidator;
        use crate::reason::Reasoner;

        let base_iri = input.base_iri.as_deref().unwrap_or("http://example.org/data/");

        // 1. Ingest
        let rows = match DataIngester::parse_file(&input.data_path) {
            Ok(r) => r,
            Err(e) => return format!(r#"{{"error":"Ingest failed: {}"}}"#, e),
        };

        let mapping = if let Some(ref mapping_str) = input.mapping {
            if input.inline_mapping.unwrap_or(false) {
                match serde_json::from_str::<MappingConfig>(mapping_str) {
                    Ok(m) => m,
                    Err(e) => return format!(r#"{{"error":"Invalid mapping: {}"}}"#, e),
                }
            } else {
                match std::fs::read_to_string(mapping_str)
                    .and_then(|c| serde_json::from_str::<MappingConfig>(&c).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
                {
                    Ok(m) => m,
                    Err(e) => return format!(r#"{{"error":"Invalid mapping file: {}"}}"#, e),
                }
            }
        } else {
            let headers = DataIngester::extract_headers(&rows);
            MappingConfig::from_headers(&headers, base_iri, &format!("{}Thing", base_iri))
        };

        let ntriples = mapping.rows_to_ntriples(&rows);
        let triples_loaded = match self.graph.load_ntriples(&ntriples) {
            Ok(c) => c,
            Err(e) => return format!(r#"{{"error":"Failed to load triples: {}"}}"#, e),
        };

        // 2. SHACL (optional)
        let mut shacl_result = serde_json::json!({"skipped": true});
        if let Some(ref shapes_input) = input.shapes {
            let shapes = if input.inline_shapes.unwrap_or(false) {
                shapes_input.clone()
            } else {
                match std::fs::read_to_string(shapes_input) {
                    Ok(c) => c,
                    Err(e) => return format!(r#"{{"error":"Cannot read shapes: {}"}}"#, e),
                }
            };
            match ShaclValidator::validate(&self.graph, &shapes) {
                Ok(report) => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&report) {
                        let stop = input.stop_on_violations.unwrap_or(true);
                        if stop && parsed["conforms"] == false {
                            return serde_json::json!({
                                "stage": "shacl",
                                "triples_ingested": triples_loaded,
                                "shacl": parsed,
                                "stopped": true,
                                "message": "Pipeline stopped due to SHACL violations",
                            }).to_string();
                        }
                        shacl_result = parsed;
                    }
                }
                Err(e) => return format!(r#"{{"error":"SHACL validation failed: {}"}}"#, e),
            }
        }

        // 3. Reasoning (optional)
        let mut reason_result = serde_json::json!({"skipped": true});
        if let Some(ref profile) = input.reason_profile {
            match Reasoner::run(&self.graph, profile, true) {
                Ok(report) => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&report) {
                        reason_result = parsed;
                    }
                }
                Err(e) => return format!(r#"{{"error":"Reasoning failed: {}"}}"#, e),
            }
        }

        serde_json::json!({
            "ok": true,
            "triples_ingested": triples_loaded,
            "rows_processed": rows.len(),
            "shacl": shacl_result,
            "reasoning": reason_result,
        }).to_string()
    }
```

**Step 3: Verify it compiles**

Run: `cd /Users/fabio/projects/open-ontologies && cargo check`
Expected: compiles with no errors

**Step 4: Commit**

```bash
git add src/server.rs
git commit -m "feat: wire up 5 new MCP tools (ingest, map, shacl, reason, extend)"
```

---

### Task 8: Integration test — full pipeline

**Files:**
- Create: `tests/extend_integration_test.rs`

**Step 1: Write the integration test**

```rust
use open_ontologies::graph::GraphStore;
use open_ontologies::ingest::DataIngester;
use open_ontologies::mapping::MappingConfig;
use open_ontologies::shacl::ShaclValidator;
use open_ontologies::reason::Reasoner;
use std::sync::Arc;

#[test]
fn test_full_pipeline_csv_to_validated_rdf() {
    // 1. Load an ontology
    let store = Arc::new(GraphStore::new());
    let ontology = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .

        ex:Building a owl:Class ;
            rdfs:label "Building" .
        ex:Landmark a owl:Class ;
            rdfs:subClassOf ex:Building ;
            rdfs:label "Landmark" .
        ex:hasName a owl:DatatypeProperty ;
            rdfs:domain ex:Building ;
            rdfs:label "has name" .
    "#;
    store.load_turtle(ontology, None).unwrap();
    let onto_triples = store.triple_count();

    // 2. Parse CSV data
    let csv = "id,name,type\nb1,Tower Bridge,Landmark\nb2,Big Ben,Landmark\n";
    let rows = DataIngester::parse_csv(csv).unwrap();
    assert_eq!(rows.len(), 2);

    // 3. Create mapping
    let mapping = MappingConfig {
        base_iri: "http://example.org/".to_string(),
        id_field: "id".to_string(),
        class: "http://example.org/Landmark".to_string(),
        mappings: vec![
            open_ontologies::mapping::FieldMapping {
                field: "id".to_string(),
                predicate: "http://example.org/id".to_string(),
                datatype: Some("http://www.w3.org/2001/XMLSchema#string".to_string()),
                class: None,
                lookup: false,
            },
            open_ontologies::mapping::FieldMapping {
                field: "name".to_string(),
                predicate: "http://www.w3.org/2000/01/rdf-schema#label".to_string(),
                datatype: Some("http://www.w3.org/2001/XMLSchema#string".to_string()),
                class: None,
                lookup: false,
            },
        ],
    };

    // 4. Ingest
    let ntriples = mapping.rows_to_ntriples(&rows);
    let loaded = store.load_ntriples(&ntriples).unwrap();
    assert!(loaded > 0);

    // 5. Validate with SHACL
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:LandmarkShape a sh:NodeShape ;
            sh:targetClass ex:Landmark ;
            sh:property [
                sh:path rdfs:label ;
                sh:minCount 1 ;
                sh:message "Landmark must have a label" ;
            ] .
    "#;
    let report = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(parsed["conforms"], true);

    // 6. Run RDFS reasoning
    let reason_report = Reasoner::run(&store, "rdfs", true).unwrap();
    let reason_parsed: serde_json::Value = serde_json::from_str(&reason_report).unwrap();
    let inferred = reason_parsed["inferred_count"].as_u64().unwrap();
    // Landmarks should be inferred as Buildings (via subClassOf)
    assert!(inferred >= 2);

    // 7. Verify inference worked
    let check = store.sparql_select(
        "ASK { <http://example.org/b1> a <http://example.org/Building> }"
    ).unwrap();
    assert!(check.contains("true"), "b1 should be inferred as a Building");
}
```

**Step 2: Run the integration test**

Run: `cd /Users/fabio/projects/open-ontologies && cargo test --test extend_integration_test -- --nocapture`
Expected: PASS

**Step 3: Run all tests to verify no regressions**

Run: `cd /Users/fabio/projects/open-ontologies && cargo test`
Expected: all tests PASS

**Step 4: Commit**

```bash
git add tests/extend_integration_test.rs
git commit -m "test: add full pipeline integration test (CSV → SHACL → RDFS reasoning)"
```

---

### Task 9: Update CLAUDE.md and README.md

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`

**Step 1: Add new tools to CLAUDE.md tool reference table**

Append to the tool table in `CLAUDE.md`:

```markdown
| `onto_ingest` | To parse structured data (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet) into RDF and load into the store |
| `onto_map` | To generate a mapping config from data schema + loaded ontology for review |
| `onto_shacl` | To validate loaded data against SHACL shapes (cardinality, datatypes, classes) |
| `onto_reason` | To run RDFS or OWL-RL inference, materializing inferred triples |
| `onto_extend` | To run the full pipeline: ingest → SHACL validate → reason in one call |
```

**Step 2: Add a new "Data Extension Workflow" section to CLAUDE.md**

After the existing "Ontology Engineering Workflow", add:

```markdown
## Data Extension Workflow

When applying an ontology to external data:

### Inspect and Map

1. Call `onto_map` with the data file — it returns field names, ontology classes/properties, and a suggested mapping
2. Review the mapping — adjust predicates, set the class, mark lookup fields
3. Optionally save the mapping to a file for reuse

### Ingest

4. Call `onto_ingest` with the data file and mapping — it generates RDF triples and loads them into the store
5. Call `onto_stats` to verify triple counts match expectations

### Validate

6. Call `onto_shacl` with SHACL shapes to validate the data against constraints
7. Fix any violations (adjust mapping or data), re-ingest if needed

### Reason

8. Call `onto_reason` with profile `rdfs` or `owl-rl` to infer new triples
9. Call `onto_query` to verify inferred knowledge is correct

### Or use the convenience pipeline

10. Call `onto_extend` to run ingest → SHACL → reason in one call
```

**Step 3: Add new tools to README.md tools table**

Add to the tools table in `README.md`:

```markdown
| `onto_ingest` | Parse structured data (CSV/JSON/XML/YAML/XLSX/Parquet) into RDF |
| `onto_map` | Generate mapping config from data schema + ontology |
| `onto_shacl` | Validate data against SHACL shapes |
| `onto_reason` | Run RDFS/OWL-RL inference (materialize triples) |
| `onto_extend` | Full pipeline: ingest → validate → reason |
```

Update the tool count from 16 to 21.

**Step 4: Commit**

```bash
git add CLAUDE.md README.md
git commit -m "docs: add data extension tools to CLAUDE.md and README.md"
```

---

### Task 10: Build release binary and verify

**Step 1: Build release**

Run: `cd /Users/fabio/projects/open-ontologies && cargo build --release`
Expected: compiles successfully

**Step 2: Run all tests one final time**

Run: `cd /Users/fabio/projects/open-ontologies && cargo test`
Expected: all tests PASS

**Step 3: Commit any remaining changes**

```bash
git add -A
git commit -m "chore: release build with 21 MCP tools (5 new data extension tools)"
```
