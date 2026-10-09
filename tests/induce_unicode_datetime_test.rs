mod common;

use open_ontologies::induce::{Dt, induce};
use open_ontologies::ingest::DataIngester;

const PRODUCTS: &str = "id,name\nproduct-1,Tシャツ半袖綿素材\nproduct-2,シャツ半袖綿素材\nproduct-3,T-shirt cotton short sleeve\n";

#[test]
fn ordinary_multilingual_product_names_induce_a_string_property() {
    let rows = DataIngester::parse_csv(PRODUCTS).unwrap();
    let headers = vec!["id".into(), "name".into()];
    let inferred = induce(&rows, &headers, "Products", "https://example.org/products/");
    assert_eq!(inferred.rows, 3);
    assert_eq!(inferred.id_column, "id");
    assert_eq!(inferred.columns.len(), 1);
    assert_eq!(inferred.columns[0].property, "name");
    assert_eq!(inferred.columns[0].datatype, Dt::String);
    let triples = inferred.mapping.rows_to_ntriples(&rows);
    assert!(triples.contains("Tシャツ半袖綿素材"));
    assert!(triples.contains("シャツ半袖綿素材"));
    assert!(triples.contains("T-shirt cotton short sleeve"));
}

#[test]
fn datetime_probe_rejects_unicode_strings_without_changing_ascii_checks() {
    for value in [
        "Tシャツ半袖綿素材",
        "2026-10-06T午後一時頃",
        "2026-10-06T13:午後一時",
        "2026-10-06T13:14:午後",
        "2026-10-06T📦発送準備",
    ] {
        assert!(!Dt::DateTime.accepts(value), "{value}");
    }
    for value in [
        "2026-10-06T13:14:15",
        "2026-10-06T13:14:15.123",
        "2026-10-06T13:14:15Z",
        "2026-10-06T13:14:15+02:00",
        "2026-10-06T13:14:15-04:00",
    ] {
        assert!(Dt::DateTime.accepts(value), "{value}");
    }
    for value in [
        "2026-10-06",
        "T-shirt cotton short sleeve",
        "2026-10-06T13:14",
        "2026-10-06T13141500",
    ] {
        assert!(!Dt::DateTime.accepts(value), "{value}");
    }
}

#[test]
fn batch_induction_accepts_a_valid_csv_with_mixed_japanese_product_names() {
    let _gate = common::exec_gate();
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("products.csv");
    let script = tmp.path().join("commands.txt");
    std::fs::write(&source, PRODUCTS).unwrap();
    std::fs::write(&script, format!("induce {} --no-load\n", source.display())).unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .args(["--no-connect", "--data-dir"])
        .arg(tmp.path())
        .arg("batch")
        .arg(script)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let output: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(output["result"]["ok"], true);
    assert_eq!(output["result"]["rows"], 3);
    assert_eq!(output["result"]["id_column"], "id");
    assert_eq!(output["result"]["columns"][0]["datatype"], "string");
}
