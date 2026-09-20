//! One sheet in, one ontology out, and the shapes it induces can FAIL.
//!
//! The rows satisfy their own induced shapes by construction, which is why a
//! clean run over them proves nothing. This file therefore checks the other
//! direction as well: change one cell so it breaks what the sheet implied and
//! the induced shape must report it. A gate that cannot fail is not a gate.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use open_ontologies::graph::GraphStore;
use open_ontologies::induce::{induce, Dt};
use open_ontologies::ingest::DataIngester;
use open_ontologies::shacl::ShaclValidator;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn sheet(rel: &str) -> (String, Vec<HashMap<String, String>>, Vec<String>) {
    let path = repo().join(rel).to_string_lossy().to_string();
    let rows = DataIngester::parse_file(&path).expect("fixture parses");
    let headers = DataIngester::headers_in_order(&path, &rows);
    (path, rows, headers)
}

#[test]
fn the_pizza_menu_becomes_one_class_with_one_multivalued_topping() {
    let (_, rows, headers) = sheet("benchmark/data/pizza-menu.csv");
    assert_eq!(headers[0], "name", "the file's first column, not the alphabetical one");
    let i = induce(&rows, &headers, "pizza-menu", "http://example.org/menu/");
    assert_eq!(i.class, "PizzaMenu");
    assert_eq!(i.id_column, "name");
    assert!(!i.id_synthesised);
    assert_eq!(i.columns.len(), 1, "seven topping columns are one property: {:?}", i.columns.iter().map(|c| &c.property).collect::<Vec<_>>());
    let t = &i.columns[0];
    assert_eq!(t.property, "topping");
    assert_eq!(t.columns.len(), 7);
    assert_eq!(t.datatype, Dt::String);
    assert!(t.max_per_row >= 2 && t.max_per_row <= 7, "{}", t.max_per_row);
    assert!(i.shapes_ttl.contains(&format!("sh:maxCount {}", t.max_per_row)));
    assert!(t.evidence.contains("rows filled"), "{}", t.evidence);
}

#[test]
fn the_epc_sample_gets_typed_columns_from_its_values() {
    let (_, rows, headers) = sheet("benchmark/epc/epc-sample.csv");
    let i = induce(&rows, &headers, "epc-sample", "http://example.org/epc/");
    let by = |p: &str| i.columns.iter().find(|c| c.property == p).unwrap_or_else(|| panic!("no column {p}"));
    assert_eq!(by("price").datatype, Dt::Double, "the fixture writes one price as 3e+05");
    assert_eq!(by("dateoftransfer").datatype, Dt::Date);
    assert_eq!(by("year").datatype, Dt::Integer);
    assert!(by("price").observed.contains_key("min"), "observed range is reported");
    assert!(!i.shapes_ttl.contains("sh:minInclusive"), "and never constrained");
    assert!(by("propertytype").values.is_some(), "a handful of letters over 30 rows is an enumeration");
    assert!(!i.ontology_ttl.contains("a owl:FunctionalProperty"));
    assert!(i.means.starts_with("an induced ontology is a HYPOTHESIS"));
}

/// The gate can fail: the sheet's own rows conform, a mutated row does not.
#[test]
fn the_induced_shape_accepts_the_sheet_and_rejects_a_broken_row() {
    let (_, mut rows, headers) = sheet("benchmark/epc/epc-sample.csv");
    let i = induce(&rows, &headers, "epc-sample", "http://example.org/epc/");

    let store = Arc::new(GraphStore::new());
    store.load_turtle(&i.ontology_ttl, None).expect("ontology parses");
    store.load_ntriples(&i.mapping.rows_to_ntriples(&rows)).expect("rows load");
    let clean: serde_json::Value =
        serde_json::from_str(&ShaclValidator::validate(&store, &i.shapes_ttl).unwrap()).unwrap();
    assert_eq!(clean["conforms"], true, "the rows satisfy their own shape by construction: {clean}");
    assert_eq!(clean["violation_count"], 0);

    // Break one cell in a way the sheet never showed: a price that is not an integer.
    rows[0].insert("price".into(), "cheap".into());
    let broken = Arc::new(GraphStore::new());
    broken.load_turtle(&i.ontology_ttl, None).unwrap();
    broken.load_ntriples(&i.mapping.rows_to_ntriples(&rows)).unwrap();
    let r: serde_json::Value =
        serde_json::from_str(&ShaclValidator::validate(&broken, &i.shapes_ttl).unwrap()).unwrap();
    assert_eq!(r["conforms"], false, "a non-integer price must be reported: {r}");
    assert!(
        r["violations"].as_array().unwrap().iter().any(|v| {
            v["source_constraint_component"] == "http://www.w3.org/ns/shacl#DatatypeConstraintComponent"
                && v["result_path"].as_str().map(|p| p.ends_with("#price")).unwrap_or(false)
        }),
        "{r}"
    );
}

/// Batch, through the built binary: `induce FILE --out DIR` writes the four
/// files, loads the store, and `stats` afterwards sees the triples.
#[test]
fn batch_induce_writes_the_files_and_loads() {
    let tmp = tempfile::TempDir::new().unwrap();
    let out = tmp.path().join("induced");
    let script = tmp.path().join("script.txt");
    std::fs::write(
        &script,
        format!(
            "induce {} --out {}\nstats\n",
            repo().join("benchmark/data/pizza-menu.csv").display(),
            out.display()
        ),
    )
    .unwrap();
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .arg("--data-dir")
        .arg(tmp.path())
        .arg("batch")
        .arg(&script)
        .output()
        .expect("the binary runs");
    let stdout = String::from_utf8_lossy(&run.stdout);
    let lines: Vec<serde_json::Value> = stdout
        .lines()
        .filter(|l| l.starts_with('{'))
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("{e}: {l}")))
        .collect();
    assert!(lines.len() >= 2, "stdout: {stdout}\nstderr: {}", String::from_utf8_lossy(&run.stderr));
    let first = &lines[0]["result"];
    assert_eq!(first["ok"], true, "{first}");
    assert_eq!(first["class"], "PizzaMenu");
    assert!(first["loaded"]["instance_triples"].as_u64().unwrap() > 0, "{first}");
    for f in ["ontology.ttl", "shapes.ttl", "mapping.json", "induced.json"] {
        assert!(out.join(f).exists(), "{f} not written");
    }
}
