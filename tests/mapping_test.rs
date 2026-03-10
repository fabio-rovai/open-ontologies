use open_ontologies::mapping::{MappingConfig, FieldMapping};

#[test]
fn test_mapping_from_csv_headers() {
    let headers = vec!["id".to_string(), "name".to_string(), "category".to_string()];
    let config = MappingConfig::from_headers(&headers, "http://example.org/data/", "http://example.org/ont#Thing");
    assert_eq!(config.mappings.len(), 3);
    assert_eq!(config.base_iri, "http://example.org/data/");
    assert_eq!(config.class, "http://example.org/ont#Thing");
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
    assert!(triples.len() >= 3); // type + id + name
    assert!(triples.iter().any(|t| t.contains("rdf:type") || t.contains("22-rdf-syntax-ns#type")));
    assert!(triples.iter().any(|t| t.contains("Tower Bridge")));
}

#[test]
fn test_mapping_rows_to_ntriples() {
    let config = MappingConfig {
        base_iri: "http://example.org/data/".to_string(),
        id_field: "id".to_string(),
        class: "http://example.org/ont#Item".to_string(),
        mappings: vec![
            FieldMapping {
                field: "id".to_string(),
                predicate: "http://example.org/ont#id".to_string(),
                datatype: None,
                class: None,
                lookup: false,
            },
            FieldMapping {
                field: "label".to_string(),
                predicate: "http://www.w3.org/2000/01/rdf-schema#label".to_string(),
                datatype: None,
                class: None,
                lookup: false,
            },
        ],
    };
    let rows: Vec<std::collections::HashMap<String, String>> = vec![
        [("id".into(), "a1".into()), ("label".into(), "Alpha".into())].into(),
        [("id".into(), "a2".into()), ("label".into(), "Beta".into())].into(),
    ];
    let nt = config.rows_to_ntriples(&rows);
    assert!(nt.contains("Alpha"));
    assert!(nt.contains("Beta"));
    // Should have triples for both rows
    let lines: Vec<&str> = nt.lines().filter(|l| !l.is_empty()).collect();
    assert!(lines.len() >= 6); // 2 rows * (type + id + label)
}

#[test]
fn test_mapping_lookup_field_produces_iri() {
    let config = MappingConfig {
        base_iri: "http://example.org/data/".to_string(),
        id_field: "id".to_string(),
        class: "http://example.org/ont#Item".to_string(),
        mappings: vec![
            FieldMapping {
                field: "category".to_string(),
                predicate: "http://example.org/ont#category".to_string(),
                datatype: None,
                class: Some("http://example.org/ont#Category".to_string()),
                lookup: true,
            },
        ],
    };
    let row: std::collections::HashMap<String, String> = [
        ("id".into(), "x1".into()),
        ("category".into(), "Electronics".into()),
    ].into();
    let triples = config.row_to_triples(&row);
    // lookup field should produce an IRI object, not a literal
    let cat_triple = triples.iter().find(|t| t.contains("ont#category")).unwrap();
    assert!(cat_triple.contains("<http://example.org/data/Electronics>"));
    assert!(!cat_triple.contains('"'));
}

#[test]
fn test_mapping_skip_empty_values() {
    let config = MappingConfig {
        base_iri: "http://example.org/data/".to_string(),
        id_field: "id".to_string(),
        class: "http://example.org/ont#Item".to_string(),
        mappings: vec![
            FieldMapping {
                field: "name".to_string(),
                predicate: "http://example.org/ont#name".to_string(),
                datatype: None,
                class: None,
                lookup: false,
            },
        ],
    };
    let row: std::collections::HashMap<String, String> = [
        ("id".into(), "x1".into()),
        ("name".into(), "".into()),
    ].into();
    let triples = config.row_to_triples(&row);
    // Should only have the rdf:type triple, not a triple for empty name
    assert_eq!(triples.len(), 1);
}
