use std::io::Cursor;
use std::sync::Mutex;

use oxigraph::io::{RdfFormat, RdfParser, RdfSerializer};
use oxigraph::model::*;
use oxigraph::sparql::QueryResults;
use oxigraph::store::Store;

/// In-memory RDF graph store backed by Oxigraph.
pub struct GraphStore {
    store: Mutex<Store>,
}

impl Default for GraphStore {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphStore {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(Store::new().expect("Failed to create Oxigraph store")),
        }
    }

    pub fn triple_count(&self) -> usize {
        let store = self.store.lock().unwrap();
        store.len().unwrap_or(0)
    }

    pub fn load_turtle(&self, ttl: &str, base_iri: Option<&str>) -> anyhow::Result<usize> {
        let store = self.store.lock().unwrap();
        let reader = Cursor::new(ttl.as_bytes());
        let mut parser = RdfParser::from_format(RdfFormat::Turtle);
        if let Some(base) = base_iri {
            parser = parser.with_base_iri(base)?;
        }
        let quads_iter = parser.for_reader(reader);
        let mut count = 0;
        for quad in quads_iter {
            store.insert(&quad?)?;
            count += 1;
        }
        Ok(count)
    }

    /// Load RDF content in a specified format (Turtle, RDF/XML, etc.)
    pub fn load_content(&self, content: &str, format: RdfFormat) -> anyhow::Result<usize> {
        self.load_content_with_base(content, format, None)
    }

    /// Load RDF content with an optional base IRI for resolving relative IRIs.
    pub fn load_content_with_base(&self, content: &str, format: RdfFormat, base_iri: Option<&str>) -> anyhow::Result<usize> {
        let store = self.store.lock().unwrap();
        let reader = Cursor::new(content.as_bytes());
        let mut parser = RdfParser::from_format(format);
        if let Some(base) = base_iri {
            parser = parser.with_base_iri(base)?;
        }
        let parser = parser.for_reader(reader);
        let mut count = 0;
        for quad in parser {
            store.insert(&quad?)?;
            count += 1;
        }
        Ok(count)
    }

    pub fn load_file(&self, path: &str) -> anyhow::Result<usize> {
        let content = std::fs::read_to_string(path)?;
        let format = Self::detect_format(path);
        let store = self.store.lock().unwrap();
        let reader = Cursor::new(content.as_bytes());
        let parser = RdfParser::from_format(format).for_reader(reader);
        let mut count = 0;
        for quad in parser {
            store.insert(&quad?)?;
            count += 1;
        }
        Ok(count)
    }

    pub fn save_file(&self, path: &str, format: &str) -> anyhow::Result<()> {
        let content = self.serialize(format)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn validate_turtle(ttl: &str) -> anyhow::Result<usize> {
        let reader = Cursor::new(ttl.as_bytes());
        let parser = RdfParser::from_format(RdfFormat::Turtle).for_reader(reader);
        let mut count = 0;
        for quad in parser {
            quad?;
            count += 1;
        }
        Ok(count)
    }

    pub fn validate_file(path: &str) -> anyhow::Result<usize> {
        let content = std::fs::read_to_string(path)?;
        let format = Self::detect_format(path);
        let reader = Cursor::new(content.as_bytes());
        let parser = RdfParser::from_format(format).for_reader(reader);
        let mut count = 0;
        for quad in parser {
            quad?;
            count += 1;
        }
        Ok(count)
    }

    pub fn sparql_select(&self, query: &str) -> anyhow::Result<String> {
        let store = self.store.lock().unwrap();
        match store.query(query)? {
            QueryResults::Solutions(solutions) => {
                let vars: Vec<String> = solutions
                    .variables()
                    .iter()
                    .map(|v| v.as_str().to_string())
                    .collect();
                let mut rows: Vec<serde_json::Value> = Vec::new();
                for solution in solutions {
                    let solution = solution?;
                    let mut row = serde_json::Map::new();
                    for var in &vars {
                        if let Some(term) = solution.get(var.as_str()) {
                            row.insert(var.clone(), serde_json::Value::String(term.to_string()));
                        }
                    }
                    rows.push(serde_json::Value::Object(row));
                }
                Ok(serde_json::json!({"variables": vars, "results": rows}).to_string())
            }
            QueryResults::Boolean(b) => Ok(serde_json::json!({"result": b}).to_string()),
            QueryResults::Graph(triples) => {
                let mut result = Vec::new();
                for triple in triples {
                    let triple = triple?;
                    result.push(serde_json::json!({
                        "subject": triple.subject.to_string(),
                        "predicate": triple.predicate.to_string(),
                        "object": triple.object.to_string(),
                    }));
                }
                Ok(serde_json::json!({"triples": result}).to_string())
            }
        }
    }

    /// Run a SPARQL UPDATE (INSERT/DELETE) against the store.
    /// Returns the number of new triples (delta).
    pub fn sparql_update(&self, update: &str) -> anyhow::Result<usize> {
        let store = self.store.lock().unwrap();
        let before = store.len()?;
        store.update(update)?;
        let after = store.len()?;
        Ok(after.saturating_sub(before))
    }

    pub fn serialize(&self, format: &str) -> anyhow::Result<String> {
        let store = self.store.lock().unwrap();
        let rdf_format = Self::parse_format(format)?;
        let mut buf = Vec::new();
        let mut serializer = RdfSerializer::from_format(rdf_format).for_writer(&mut buf);
        for quad in store.iter() {
            let quad = quad?;
            serializer.serialize_triple(quad.as_ref())?;
        }
        drop(serializer);
        Ok(String::from_utf8(buf)?)
    }

    pub fn get_stats(&self) -> anyhow::Result<String> {
        let store = self.store.lock().unwrap();
        let total = store.len()?;

        // Count classes: explicit type declarations + implicit (subClassOf subjects/objects,
        // domain/range targets, equivalentClass). Filters out blank nodes and OWL/RDF builtins.
        let class_query = "SELECT (COUNT(DISTINCT ?c) AS ?count) WHERE {
            { ?c a <http://www.w3.org/2002/07/owl#Class> }
            UNION { ?c a <http://www.w3.org/2000/01/rdf-schema#Class> }
            UNION { ?c <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?p }
            UNION { ?p <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c }
            UNION { ?p <http://www.w3.org/2000/01/rdf-schema#domain> ?c }
            UNION { ?p <http://www.w3.org/2000/01/rdf-schema#range> ?c }
            UNION { ?c <http://www.w3.org/2002/07/owl#equivalentClass> ?p }
            FILTER(isIRI(?c)
                && ?c != <http://www.w3.org/2002/07/owl#Thing>
                && ?c != <http://www.w3.org/2002/07/owl#Nothing>
                && ?c != <http://www.w3.org/2000/01/rdf-schema#Resource>
                && ?c != <http://www.w3.org/2000/01/rdf-schema#Literal>
                && ?c != <http://www.w3.org/2000/01/rdf-schema#Class>
                && ?c != <http://www.w3.org/2002/07/owl#Class>)
        }";
        // Count properties: explicit type + implicit (subPropertyOf, domain/range subjects)
        let prop_query = "SELECT (COUNT(DISTINCT ?p) AS ?count) WHERE {
            { ?p a <http://www.w3.org/2002/07/owl#ObjectProperty> }
            UNION { ?p a <http://www.w3.org/2002/07/owl#DatatypeProperty> }
            UNION { ?p a <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property> }
            UNION { ?p <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?q }
            UNION { ?q <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?p }
            UNION { ?p <http://www.w3.org/2000/01/rdf-schema#domain> ?c }
            UNION { ?p <http://www.w3.org/2000/01/rdf-schema#range> ?c }
            FILTER(isIRI(?p)
                && !STRSTARTS(STR(?p), \"http://www.w3.org/1999/02/22-rdf-syntax-ns#\")
                && !STRSTARTS(STR(?p), \"http://www.w3.org/2000/01/rdf-schema#\")
                && !STRSTARTS(STR(?p), \"http://www.w3.org/2002/07/owl#\"))
        }";
        let individual_query = "SELECT (COUNT(DISTINCT ?i) AS ?count) WHERE { ?i a ?c . FILTER(?c != <http://www.w3.org/2002/07/owl#Class> && ?c != <http://www.w3.org/2000/01/rdf-schema#Class> && ?c != <http://www.w3.org/2002/07/owl#ObjectProperty> && ?c != <http://www.w3.org/2002/07/owl#DatatypeProperty> && ?c != <http://www.w3.org/2002/07/owl#Ontology>) }";

        let count_from_query = |q: &str| -> usize {
            let Ok(QueryResults::Solutions(solutions)) = store.query(q) else { return 0 };
            let Some(Ok(row)) = solutions.into_iter().next() else { return 0 };
            let Some(Term::Literal(lit)) = row.get("count") else { return 0 };
            lit.value().parse().unwrap_or(0)
        };

        let classes = count_from_query(class_query);
        let props = count_from_query(prop_query);
        let individuals = count_from_query(individual_query);

        Ok(serde_json::json!({
            "triples": total,
            "classes": classes,
            "object_properties": props,
            "data_properties": 0,
            "properties": props,
            "individuals": individuals
        })
        .to_string())
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        let store = self.store.lock().unwrap();
        store.clear()?;
        Ok(())
    }

    pub fn load_ntriples(&self, content: &str) -> anyhow::Result<usize> {
        let store = self.store.lock().unwrap();
        let reader = Cursor::new(content.as_bytes());
        let parser = RdfParser::from_format(RdfFormat::NTriples).for_reader(reader);
        let mut count = 0;
        for quad in parser {
            store.insert(&quad?)?;
            count += 1;
        }
        Ok(count)
    }

    pub fn snapshot(&self, format: &str) -> anyhow::Result<String> {
        self.serialize(format)
    }

    pub async fn fetch_url(url: &str) -> anyhow::Result<String> {
        let resp = reqwest::get(url).await?;
        if !resp.status().is_success() {
            anyhow::bail!("HTTP {}: {}", resp.status(), url);
        }
        Ok(resp.text().await?)
    }

    pub async fn fetch_sparql(endpoint: &str, query: &str) -> anyhow::Result<String> {
        let client = reqwest::Client::new();
        let resp = client
            .post(endpoint)
            .header("Content-Type", "application/sparql-query")
            .header("Accept", "text/turtle")
            .body(query.to_string())
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("SPARQL endpoint returned HTTP {}", resp.status());
        }
        Ok(resp.text().await?)
    }

    pub async fn push_sparql(endpoint: &str, content: &str) -> anyhow::Result<String> {
        let client = reqwest::Client::new();
        let resp = client
            .post(endpoint)
            .header("Content-Type", "application/sparql-update")
            .body(format!("INSERT DATA {{ {} }}", content))
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("SPARQL update returned HTTP {}", resp.status());
        }
        Ok(format!("Pushed to {}: HTTP {}", endpoint, resp.status()))
    }

    /// Extract all triples as (subject, predicate, object) string tuples.
    pub fn all_triples(&self) -> anyhow::Result<Vec<(String, String, String)>> {
        let store = self.store.lock().unwrap();
        let mut triples = Vec::new();
        for quad in store.iter() {
            let quad = quad?;
            let s = quad.subject.to_string();
            let p = quad.predicate.to_string();
            let o = quad.object.to_string();
            triples.push((s, p, o));
        }
        Ok(triples)
    }

    fn detect_format(path: &str) -> RdfFormat {
        if path.ends_with(".ttl") || path.ends_with(".turtle") {
            RdfFormat::Turtle
        } else if path.ends_with(".nt") || path.ends_with(".ntriples") {
            RdfFormat::NTriples
        } else if path.ends_with(".rdf") || path.ends_with(".xml") || path.ends_with(".owl") {
            RdfFormat::RdfXml
        } else if path.ends_with(".nq") {
            RdfFormat::NQuads
        } else if path.ends_with(".trig") {
            RdfFormat::TriG
        } else {
            RdfFormat::Turtle
        }
    }

    fn parse_format(name: &str) -> anyhow::Result<RdfFormat> {
        match name.to_lowercase().as_str() {
            "turtle" | "ttl" => Ok(RdfFormat::Turtle),
            "ntriples" | "nt" => Ok(RdfFormat::NTriples),
            "rdfxml" | "rdf" | "xml" | "owl" => Ok(RdfFormat::RdfXml),
            "nquads" | "nq" => Ok(RdfFormat::NQuads),
            "trig" => Ok(RdfFormat::TriG),
            _ => anyhow::bail!(
                "Unknown format: {}. Supported: turtle, ntriples, rdfxml, nquads, trig",
                name
            ),
        }
    }
}
