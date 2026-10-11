//! Invalid literal percent characters must not produce invalid mapped IRIs.
use open_ontologies::graph::GraphStore;
use open_ontologies::mapping::{FieldMapping, MappingConfig};
use std::collections::HashMap;

const BASE: &str = "https://example.org/";

#[test]
fn percent_components_parse_in_subjects_predicates_and_lookup_objects() {
    for (value, expected) in [
        ("50%", "50%25"),
        ("%", "%25"),
        ("%A", "%25A"),
        ("%G1", "%25G1"),
        ("%é", "%25é"),
        ("%%20", "%25%20"),
        ("name%20space", "name%20space"),
        ("path%2fpart", "path%2fpart"),
        ("literal%25", "literal%25"),
        ("日本語", "日本語"),
    ] {
        let mut mapping = MappingConfig::from_headers(
            &["id".into(), value.into()],
            BASE,
            &format!("{BASE}Record"),
        );
        mapping.mappings.push(FieldMapping {
            field: "lookup".into(),
            predicate: format!("{BASE}lookup"),
            datatype: None,
            class: None,
            lookup: true,
        });
        let row = HashMap::from([
            ("id".into(), value.into()),
            (value.into(), "plain literal % stays %".into()),
            ("lookup".into(), value.into()),
        ]);
        let triples = mapping.rows_to_ntriples(&[row]);
        assert!(
            triples.starts_with(&format!("<{BASE}{expected}>")),
            "{triples}"
        );
        assert!(
            triples.contains(&format!("<{BASE}ont#{expected}>")),
            "{triples}"
        );
        assert!(
            triples.contains(&format!("<{BASE}lookup> <{BASE}{expected}>")),
            "{triples}"
        );
        assert!(triples.contains("plain literal % stays %"), "{triples}");
        GraphStore::new().load_ntriples(&triples).unwrap();
    }
}
