// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

use open_ontologies::graph::GraphStore;
use open_ontologies::schema::{ColumnInfo, ForeignKey, SchemaIntrospector, TableInfo};

#[test]
fn generated_schema_turtle_accepts_quoted_sql_identifiers() {
    let tables = vec![
        TableInfo {
            name: "customer archive".into(),
            columns: vec![ColumnInfo {
                name: "id".into(),
                data_type: "integer".into(),
                is_nullable: false,
                is_primary_key: true,
            }],
            foreign_keys: vec![],
        },
        TableInfo {
            name: "order archive".into(),
            columns: vec![ColumnInfo {
                name: "customer \"id\"".into(),
                data_type: "integer".into(),
                is_nullable: true,
                is_primary_key: false,
            }],
            foreign_keys: vec![ForeignKey {
                column: "customer \"id\"".into(),
                parent_table: "customer archive".into(),
                parent_column: "id".into(),
            }],
        },
    ];

    let turtle = SchemaIntrospector::generate_turtle(&tables, "https://example.org/db/");
    assert!(turtle.contains("db:Customer%20archive a owl:Class"));
    assert!(turtle.contains("db:Order%20archive a owl:Class"));
    assert!(turtle.contains("db:order%20archive_customer%20%22id%22 a owl:ObjectProperty"));
    assert!(turtle.contains("rdfs:range db:Customer%20archive"));
    assert!(turtle.contains("rdfs:label \"customer \\\"id\\\"\" ."));
    GraphStore::validate_turtle(&turtle).expect("quoted SQL identifiers produce valid Turtle");
}
