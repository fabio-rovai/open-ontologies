use open_ontologies::graph::GraphStore;
use open_ontologies::induce::{Dt, induce};
use open_ontologies::ingest::DataIngester;
use open_ontologies::mapping::{FieldMapping, MappingConfig};

fn fixture(name: &str) -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/ingest")
        .join(name)
        .to_str()
        .unwrap()
        .to_owned()
}

#[test]
fn both_excel_calendars_preserve_datetimes_and_other_cell_types() {
    for name in ["excel-calendar-1900.xlsx", "excel-calendar-1904.xlsx"] {
        let rows = DataIngester::parse_xlsx_file(&fixture(name)).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["midnight"], "2026-10-06T00:00:00");
        assert_eq!(rows[0]["datetime"], "2026-10-06T13:14:15.123");
        assert_eq!(
            rows[0]["duration"], "1.5",
            "elapsed time is not a calendar date"
        );
        assert_eq!(
            rows[0]["number"], "46301",
            "unformatted numbers remain numbers"
        );
        assert_eq!(rows[0]["boolean"], "true");
        assert_eq!(rows[0]["empty"], "");
        assert_eq!(rows[0]["error"], "Div0");
        let headers = DataIngester::extract_headers(&rows);
        let inferred = induce(&rows, &headers, "Rows", "https://example.org/rows/");
        for field in ["midnight", "datetime"] {
            assert_eq!(
                inferred
                    .columns
                    .iter()
                    .find(|column| column.property == field)
                    .unwrap()
                    .datatype,
                Dt::DateTime
            );
        }
    }
}

#[test]
fn serial_and_iso_cells_match_csv_and_keep_an_explicit_datetime_mapping() {
    let csv = DataIngester::parse_csv("id,datetime\nrow-1,2026-10-06T13:14:15.123\n").unwrap();
    let iso = DataIngester::parse_xlsx_file(&fixture("excel-calendar-iso.xlsx")).unwrap();
    let mapping = MappingConfig {
        base_iri: "https://example.org/rows/".into(),
        id_field: "id".into(),
        class: "https://example.org/Row".into(),
        mappings: vec![FieldMapping {
            field: "datetime".into(),
            predicate: "https://example.org/when".into(),
            datatype: Some("http://www.w3.org/2001/XMLSchema#dateTime".into()),
            class: None,
            lookup: false,
        }],
    };
    for name in ["excel-calendar-1900.xlsx", "excel-calendar-1904.xlsx"] {
        let rows = DataIngester::parse_xlsx_file(&fixture(name)).unwrap();
        assert_eq!(rows[0]["datetime"], csv[0]["datetime"]);
        assert_eq!(rows[0]["datetime"], iso[0]["datetime"]);
        let triples = mapping.rows_to_ntriples(&rows);
        let store = GraphStore::new();
        store.load_ntriples(&triples).unwrap();
        let query = store.sparql_select(
            "SELECT (DATATYPE(?value) AS ?datatype) WHERE { ?s <https://example.org/when> ?value }",
        ).unwrap();
        assert!(
            query.contains("http://www.w3.org/2001/XMLSchema#dateTime"),
            "{query}"
        );
    }
}
