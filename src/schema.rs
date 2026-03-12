/// Database schema introspection and OWL generation.

pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
    pub foreign_keys: Vec<ForeignKey>,
}

pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
}

pub struct ForeignKey {
    pub column: String,
    pub parent_table: String,
    pub parent_column: String,
}

pub struct SchemaIntrospector;

impl SchemaIntrospector {
    /// Convert SQL type name to XSD datatype.
    pub fn sql_to_xsd(sql_type: &str) -> &'static str {
        match sql_type.to_lowercase().as_str() {
            "integer" | "int" | "bigint" | "smallint" | "int4" | "int8" | "int2" | "serial" | "bigserial" => "xsd:integer",
            "numeric" | "decimal" | "real" | "double precision" | "float4" | "float8" => "xsd:decimal",
            "boolean" | "bool" => "xsd:boolean",
            "date" => "xsd:date",
            "timestamp" | "timestamptz" | "timestamp without time zone" | "timestamp with time zone" => "xsd:dateTime",
            "bytea" | "blob" => "xsd:hexBinary",
            _ => "xsd:string",
        }
    }

    /// Convert snake_case table name to PascalCase class name.
    pub fn table_to_class(name: &str) -> String {
        name.split('_')
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().to_string() + &chars.collect::<String>(),
                }
            })
            .collect()
    }

    /// Generate OWL Turtle from introspected schema.
    pub fn generate_turtle(tables: &[TableInfo], base_iri: &str) -> String {
        let mut ttl = String::new();
        ttl.push_str("@prefix owl: <http://www.w3.org/2002/07/owl#> .\n");
        ttl.push_str("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n");
        ttl.push_str("@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n");
        ttl.push_str(&format!("@prefix db: <{}> .\n\n", base_iri));

        // Build FK lookup: (table, column) → parent_table
        let fk_map: std::collections::HashMap<(String, String), &ForeignKey> = tables.iter()
            .flat_map(|t| t.foreign_keys.iter().map(move |fk| ((t.name.clone(), fk.column.clone()), fk)))
            .collect();

        for table in tables {
            let class = Self::table_to_class(&table.name);
            ttl.push_str(&format!("db:{} a owl:Class ;\n    rdfs:label \"{}\" .\n\n", class, class));

            for col in &table.columns {
                let prop_name = format!("{}_{}", table.name, col.name);

                if let Some(fk) = fk_map.get(&(table.name.clone(), col.name.clone())) {
                    // Foreign key → ObjectProperty
                    let parent_class = Self::table_to_class(&fk.parent_table);
                    ttl.push_str(&format!("db:{} a owl:ObjectProperty ;\n", prop_name));
                    ttl.push_str(&format!("    rdfs:domain db:{} ;\n", class));
                    ttl.push_str(&format!("    rdfs:range db:{} ;\n", parent_class));
                    ttl.push_str(&format!("    rdfs:label \"{}\" .\n\n", col.name));
                } else {
                    // Regular column → DatatypeProperty
                    let xsd = Self::sql_to_xsd(&col.data_type);
                    if col.is_primary_key {
                        ttl.push_str(&format!("db:{} a owl:DatatypeProperty , owl:FunctionalProperty ;\n", prop_name));
                    } else {
                        ttl.push_str(&format!("db:{} a owl:DatatypeProperty ;\n", prop_name));
                    }
                    ttl.push_str(&format!("    rdfs:domain db:{} ;\n", class));
                    ttl.push_str(&format!("    rdfs:range {} ;\n", xsd));
                    ttl.push_str(&format!("    rdfs:label \"{}\" .\n\n", col.name));
                }

                // NOT NULL → cardinality restriction
                if !col.is_nullable {
                    ttl.push_str(&format!("db:{} rdfs:subClassOf [\n", class));
                    ttl.push_str("    a owl:Restriction ;\n");
                    ttl.push_str(&format!("    owl:onProperty db:{} ;\n", prop_name));
                    ttl.push_str("    owl:minCardinality 1\n");
                    ttl.push_str("] .\n\n");
                }
            }
        }

        ttl
    }

    /// Connect to postgres, introspect schema, return TableInfo vec.
    #[cfg(feature = "postgres")]
    pub async fn introspect_postgres(connection_string: &str) -> anyhow::Result<Vec<TableInfo>> {
        use sqlx::postgres::PgPoolOptions;
        use sqlx::Row;

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(connection_string)
            .await?;

        // Get tables
        let table_rows = sqlx::query(
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_type = 'BASE TABLE'"
        ).fetch_all(&pool).await?;

        let mut tables = Vec::new();

        for trow in &table_rows {
            let table_name: String = trow.get("table_name");

            // Get columns
            let col_rows = sqlx::query(
                "SELECT column_name, data_type, is_nullable FROM information_schema.columns WHERE table_schema = 'public' AND table_name = $1 ORDER BY ordinal_position"
            ).bind(&table_name).fetch_all(&pool).await?;

            // Get primary keys
            let pk_rows = sqlx::query(
                "SELECT kcu.column_name FROM information_schema.table_constraints tc JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema WHERE tc.constraint_type = 'PRIMARY KEY' AND tc.table_name = $1"
            ).bind(&table_name).fetch_all(&pool).await?;

            let pk_cols: Vec<String> = pk_rows.iter().map(|r| r.get("column_name")).collect();

            let columns: Vec<ColumnInfo> = col_rows.iter().map(|r| {
                let name: String = r.get("column_name");
                let data_type: String = r.get("data_type");
                let nullable: String = r.get("is_nullable");
                ColumnInfo {
                    is_primary_key: pk_cols.contains(&name),
                    name,
                    data_type,
                    is_nullable: nullable == "YES",
                }
            }).collect();

            // Get foreign keys
            let fk_rows = sqlx::query(
                "SELECT kcu.column_name AS child_column, ccu.table_name AS parent_table, ccu.column_name AS parent_column FROM information_schema.table_constraints tc JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema JOIN information_schema.constraint_column_usage ccu ON tc.constraint_name = ccu.constraint_name WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_name = $1"
            ).bind(&table_name).fetch_all(&pool).await?;

            let foreign_keys: Vec<ForeignKey> = fk_rows.iter().map(|r| ForeignKey {
                column: r.get("child_column"),
                parent_table: r.get("parent_table"),
                parent_column: r.get("parent_column"),
            }).collect();

            tables.push(TableInfo { name: table_name, columns, foreign_keys });
        }

        pool.close().await;
        Ok(tables)
    }
}
