#![cfg(feature = "postgres")]

use open_ontologies::schema::{TableInfo, ColumnInfo, ForeignKey, SchemaIntrospector};

#[test]
fn test_generate_turtle_single_table() {
    let tables = vec![TableInfo {
        name: "users".into(),
        columns: vec![
            ColumnInfo { name: "id".into(), data_type: "integer".into(), is_nullable: false, is_primary_key: true },
            ColumnInfo { name: "name".into(), data_type: "varchar".into(), is_nullable: false, is_primary_key: false },
            ColumnInfo { name: "email".into(), data_type: "varchar".into(), is_nullable: true, is_primary_key: false },
        ],
        foreign_keys: vec![],
    }];

    let turtle = SchemaIntrospector::generate_turtle(&tables, "http://example.org/db/");
    assert!(turtle.contains("db:Users a owl:Class"));
    assert!(turtle.contains("db:users_name a owl:DatatypeProperty"));
    assert!(turtle.contains("xsd:string"));
    assert!(turtle.contains("owl:minCardinality"));  // NOT NULL → minCard 1
    assert!(turtle.contains("owl:FunctionalProperty"));  // PK
}

#[test]
fn test_generate_turtle_foreign_key() {
    let tables = vec![
        TableInfo {
            name: "users".into(),
            columns: vec![
                ColumnInfo { name: "id".into(), data_type: "integer".into(), is_nullable: false, is_primary_key: true },
            ],
            foreign_keys: vec![],
        },
        TableInfo {
            name: "orders".into(),
            columns: vec![
                ColumnInfo { name: "id".into(), data_type: "integer".into(), is_nullable: false, is_primary_key: true },
                ColumnInfo { name: "user_id".into(), data_type: "integer".into(), is_nullable: false, is_primary_key: false },
            ],
            foreign_keys: vec![ForeignKey {
                column: "user_id".into(),
                parent_table: "users".into(),
                parent_column: "id".into(),
            }],
        },
    ];

    let turtle = SchemaIntrospector::generate_turtle(&tables, "http://example.org/db/");
    assert!(turtle.contains("db:orders_user_id a owl:ObjectProperty"));
    assert!(turtle.contains("rdfs:range db:Users"));
}

#[test]
fn test_sql_type_to_xsd() {
    assert_eq!(SchemaIntrospector::sql_to_xsd("integer"), "xsd:integer");
    assert_eq!(SchemaIntrospector::sql_to_xsd("varchar"), "xsd:string");
    assert_eq!(SchemaIntrospector::sql_to_xsd("boolean"), "xsd:boolean");
    assert_eq!(SchemaIntrospector::sql_to_xsd("timestamp"), "xsd:dateTime");
    assert_eq!(SchemaIntrospector::sql_to_xsd("numeric"), "xsd:decimal");
    assert_eq!(SchemaIntrospector::sql_to_xsd("date"), "xsd:date");
    assert_eq!(SchemaIntrospector::sql_to_xsd("bytea"), "xsd:hexBinary");
    assert_eq!(SchemaIntrospector::sql_to_xsd("unknown_type"), "xsd:string");
}

#[test]
fn test_generate_turtle_not_null_cardinality() {
    let tables = vec![TableInfo {
        name: "users".into(),
        columns: vec![
            ColumnInfo { name: "id".into(), data_type: "integer".into(), is_nullable: false, is_primary_key: true },
            ColumnInfo { name: "email".into(), data_type: "varchar".into(), is_nullable: false, is_primary_key: false },
        ],
        foreign_keys: vec![],
    }];

    let turtle = SchemaIntrospector::generate_turtle(&tables, "http://example.org/db/");
    assert!(turtle.contains("owl:minCardinality"));
}

#[test]
fn test_table_name_to_class() {
    assert_eq!(SchemaIntrospector::table_to_class("users"), "Users");
    assert_eq!(SchemaIntrospector::table_to_class("order_items"), "OrderItems");
    assert_eq!(SchemaIntrospector::table_to_class("product"), "Product");
}
