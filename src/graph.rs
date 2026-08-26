use std::io::Cursor;
use std::path::Path;
use std::sync::Mutex;

use oxigraph::io::{JsonLdProfileSet, RdfFormat, RdfParser, RdfSerializer};
use oxigraph::model::*;
use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;

/// What a parse produced. `statements` counts parser events; `triples` counts
/// distinct triples, which is the size of the resulting graph. They differ
/// whenever the source repeats a statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationCounts {
    pub statements: usize,
    pub triples: usize,
}

/// Optional HTTP authentication for remote SPARQL endpoints.
///
/// Enterprise triple stores gate their SPARQL Protocol endpoints behind auth:
/// Stardog and Ontotext GraphDB accept HTTP Basic; token-secured deployments
/// accept a Bearer token. Open stores (Apache Jena/Fuseki, Eclipse RDF4J,
/// public Virtuoso) need none — leave this empty.
#[derive(Default, Clone)]
pub struct SparqlAuth {
    /// HTTP Basic credentials as (username, password).
    pub basic: Option<(String, String)>,
    /// Bearer token (takes precedence over `basic` if both are set).
    pub bearer: Option<String>,
}

impl SparqlAuth {
    /// Build from optional username/password/token (e.g. tool inputs).
    /// Returns a no-auth value when all are absent.
    pub fn from_parts(
        username: Option<String>,
        password: Option<String>,
        token: Option<String>,
    ) -> Self {
        let basic = match (username, password) {
            (Some(u), Some(p)) => Some((u, p)),
            (Some(u), None) => Some((u, String::new())),
            _ => None,
        };
        SparqlAuth { basic, bearer: token }
    }

    /// Apply the configured auth to a request builder.
    fn apply(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(t) = &self.bearer {
            rb.bearer_auth(t)
        } else if let Some((u, p)) = &self.basic {
            rb.basic_auth(u, Some(p))
        } else {
            rb
        }
    }
}

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

    /// Open a RocksDB-backed persistent store at `path`, creating it if missing.
    ///
    /// Oxigraph allows only one read-write handle per directory; opening the
    /// same path from a second process will fail. Sandbox stores throughout
    /// the codebase keep using [`GraphStore::new`] — only the main graph
    /// should ever be persistent.
    pub fn open_persistent(path: &Path) -> anyhow::Result<Self> {
        std::fs::create_dir_all(path).map_err(|e| {
            anyhow::anyhow!(
                "failed to create persistent triplestore directory {}: {e}",
                path.display()
            )
        })?;
        let store = Store::open(path).map_err(|e| {
            anyhow::anyhow!(
                "failed to open persistent Oxigraph store at {}: {e}",
                path.display()
            )
        })?;
        Ok(Self {
            store: Mutex::new(store),
        })
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
        // Parse the whole document BEFORE touching the store. Streaming
        // inserts left every quad before the first syntax error in place, so
        // a failed load produced a silently partial graph that looked exactly
        // like a small one (issue #93). All or nothing.
        let quads: Vec<_> = parser
            .for_reader(reader)
            .collect::<Result<_, _>>()?;
        // Report triples actually added, not parse events. The store is a set, so
        // re-inserting a statement it already holds changes nothing and must not be
        // counted as a load.
        let before = store.len().unwrap_or(0);
        for quad in &quads {
            store.insert(quad)?;
        }
        Ok(store.len().unwrap_or(before).saturating_sub(before))
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
        // All or nothing: see load_turtle (issue #93).
        let quads: Vec<_> = parser
            .for_reader(reader)
            .collect::<Result<_, _>>()?;
        // Report triples actually added, not parse events. The store is a set, so
        // re-inserting a statement it already holds changes nothing and must not be
        // counted as a load.
        let before = store.len().unwrap_or(0);
        for quad in &quads {
            store.insert(quad)?;
        }
        Ok(store.len().unwrap_or(before).saturating_sub(before))
    }

    pub fn load_file(&self, path: &str) -> anyhow::Result<usize> {
        let content = std::fs::read_to_string(path)?;
        let format = Self::detect_format_sniffed(path, &content);
        let store = self.store.lock().unwrap();
        let reader = Cursor::new(content.as_bytes());

        // A document's own location is its default base, per RFC 3986. Without
        // it, any file using relative IRIs fails to parse at all, which is
        // most published RDF/XML: LUBM's generated data would not load a
        // single triple before this.
        let base = std::fs::canonicalize(path)
            .ok()
            .and_then(|abs| abs.to_str().map(|s| format!("file://{s}")));
        // All or nothing: see load_turtle (issue #93).
        let mut parser = RdfParser::from_format(format);
        if let Some(p) = base.as_ref().and_then(|b| parser.clone().with_base_iri(b).ok()) {
            parser = p;
        }
        let quads: Vec<_> = parser
            .for_reader(reader)
            .collect::<Result<_, _>>()?;
        // Report triples actually added, not parse events. The store is a set, so
        // re-inserting a statement it already holds changes nothing and must not be
        // counted as a load.
        let before = store.len().unwrap_or(0);
        for quad in &quads {
            store.insert(quad)?;
        }
        Ok(store.len().unwrap_or(before).saturating_sub(before))
    }

    pub fn save_file(&self, path: &str, format: &str) -> anyhow::Result<()> {
        let content = self.serialize(format)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn validate_turtle(ttl: &str) -> anyhow::Result<ValidationCounts> {
        let reader = Cursor::new(ttl.as_bytes());
        let parser = RdfParser::from_format(RdfFormat::Turtle).for_reader(reader);
        Self::count_parsed(parser)
    }

    pub fn validate_file(path: &str) -> anyhow::Result<ValidationCounts> {
        let content = std::fs::read_to_string(path)?;
        let format = Self::detect_format_sniffed(path, &content);
        let reader = Cursor::new(content.as_bytes());
        let parser = RdfParser::from_format(format).for_reader(reader);
        Self::count_parsed(parser)
    }

    /// Count what a parser produced, distinguishing statements from triples.
    ///
    /// An RDF graph is a set, so a statement repeated in the source contributes
    /// one triple and not two. Reporting the parse-event count as a triple count
    /// overstates any generated document that repeats a statement, which real
    /// serialisers do constantly: emitting `?lib a :Library` once per record is
    /// ordinary practice and inflated one 16.7 MB file by 6.7 per cent.
    fn count_parsed<I>(parser: I) -> anyhow::Result<ValidationCounts>
    where
        I: IntoIterator<Item = Result<Quad, oxigraph::io::RdfParseError>>,
    {
        let mut statements = 0usize;
        let mut seen = std::collections::HashSet::new();
        for quad in parser {
            let quad = quad?;
            statements += 1;
            seen.insert(quad);
        }
        Ok(ValidationCounts {
            statements,
            triples: seen.len(),
        })
    }

    /// Run a SELECT over the store's default graph.
    ///
    /// **Which of the two you want depends on who wrote the query.** A query
    /// someone typed belongs here: they chose the dataset by writing `GRAPH`
    /// or not writing it, and widening it under them would change the meaning
    /// of what they wrote. A query this codebase authored to ask a question
    /// about the store belongs in [`Self::sparql_select_union`], because the
    /// answer to "what does this store declare" or "which instances are
    /// there" must not depend on the file format the triples arrived in.
    ///
    /// Getting that backwards does not look like a bug. It looks like a clean
    /// report over a store that holds nothing, which is how it survived in
    /// four separate tools at once (#108). `tests/serialisation_invariance_test.rs`
    /// is where a tool's answer is pinned against both serialisations.
    pub fn sparql_select(&self, query: &str) -> anyhow::Result<String> {
        self.select_with_dataset(query, false)
    }

    /// Run a SELECT whose default graph is the union of every graph in the
    /// store, named graphs included.
    ///
    /// The plain `sparql_select` leaves the evaluator on its default dataset
    /// specification, which is the store's default graph alone. That is the
    /// right dataset for a query someone wrote, since a caller who wants a
    /// named graph writes `GRAPH`. It is the wrong one for a tool that asks a
    /// question about the store rather than about a graph, because the answer
    /// then depends on which serialisation the data arrived in: the same
    /// triples loaded from Turtle are visible and loaded from TriG are not.
    ///
    /// `GRAPH ?g` still ranges over the named graphs here, so a query written
    /// against the plain form keeps its meaning under this one.
    pub fn sparql_select_union(&self, query: &str) -> anyhow::Result<String> {
        self.select_with_dataset(query, true)
    }

    fn select_with_dataset(
        &self,
        query: &str,
        union_default_graph: bool,
    ) -> anyhow::Result<String> {
        let store = self.store.lock().unwrap();
        let mut prepared = SparqlEvaluator::new().parse_query(query)?;
        if union_default_graph {
            prepared.dataset_mut().set_default_graph_as_union();
        }
        match prepared.on_store(&store).execute()? {
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

    /// Canonicalise the store's blank nodes via RDFC 1.0 (W3C Recommendation,
    /// 21 May 2024) using SHA-256, returning a NEW `GraphStore` whose blank
    /// nodes have deterministic `_:c14n<n>` identifiers derived from the graph
    /// structure.
    ///
    /// This is the principled successor to per-callsite "filter `_:` IRIs out
    /// of the SPARQL result set" — for any operation that depends on stable
    /// identity across reparses (drift detection, hashing, signature
    /// comparison), canonicalisation preserves the semantic content of
    /// anonymous restriction classes / quoted axioms instead of dropping them.
    ///
    /// **Warning:** per the W3C spec, canonical IDs are a function of the
    /// whole graph. Mutating one quad can shift many bnode IDs, so this
    /// is poorly suited to producing minimal-diff outputs over arbitrary
    /// edits. For drift detection specifically, the existing rename-pairing
    /// logic in `drift.rs::detect()` will re-match shifted IDs via the
    /// label/domain/range/hierarchy/individual signal ensemble, so the
    /// net result is more informative than the previous "filter and forget"
    /// approach (PR #14, @rustforrecess) that dropped bnode content entirely.
    pub fn canonicalize_blank_nodes(&self) -> anyhow::Result<GraphStore> {
        use oxigraph::model::dataset::{CanonicalizationAlgorithm, CanonicalizationHashAlgorithm};
        use oxigraph::model::Dataset;

        let store = self.store.lock().unwrap();
        let mut dataset = Dataset::new();
        for quad in store.iter() {
            let q = quad?;
            dataset.insert(&q);
        }
        drop(store);

        dataset.canonicalize(CanonicalizationAlgorithm::Rdfc10 {
            hash_algorithm: CanonicalizationHashAlgorithm::Sha256,
        });

        let new_gs = GraphStore::new();
        {
            let new_store = new_gs.store.lock().unwrap();
            for quad in dataset.iter() {
                new_store.insert(quad)?;
            }
        }
        Ok(new_gs)
    }

    pub fn serialize(&self, format: &str) -> anyhow::Result<String> {
        let store = self.store.lock().unwrap();
        let rdf_format = Self::parse_format(format)?;
        // Dataset formats carry the graph name; every other format is a single
        // RDF graph. `serialize_triple` drops the graph name, flattening a quad
        // from a named graph into the default graph. That is the only thing a
        // triple format can do, but for the dataset formats it silently
        // discarded the named-graph structure that temporal assertions live in
        // (issue #95): a TriG save/reload round trip lost every
        // `validFrom`/`validTo` binding. `serialize_quad` keeps the graph name,
        // and for the triple formats we keep flattening. `supports_datasets`
        // owns the list upstream, so a format added later (JSON-LD is already
        // in it) is classified correctly rather than silently flattened.
        let carries_graph_name = rdf_format.supports_datasets();
        let mut buf = Vec::new();
        let mut serializer = RdfSerializer::from_format(rdf_format).for_writer(&mut buf);
        for quad in store.iter() {
            let quad = quad?;
            if carries_graph_name {
                serializer.serialize_quad(quad.as_ref())?;
            } else {
                serializer.serialize_triple(quad.as_ref())?;
            }
        }
        // `finish()` writes the final terminator (e.g. the trailing `.` on the
        // last Turtle triple, or `</rdf:RDF>` for RDF/XML). Dropping the
        // serializer skips this step, which produced truncated, unparseable
        // output — see `convert` → `drift` round-trip on the Pizza ontology.
        serializer.finish()?;
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
            let Ok(prepared) = SparqlEvaluator::new().parse_query(q) else { return 0 };
            let Ok(QueryResults::Solutions(solutions)) = prepared
                .on_store(&store)
                .execute()
            else { return 0 };
            let Some(Ok(row)) = solutions.into_iter().next() else { return 0 };
            let Some(Term::Literal(lit)) = row.get("count") else { return 0 };
            lit.value().parse().unwrap_or(0)
        };

        // Typed subsets: object vs datatype properties. The broad `prop_query`
        // above also counts rdf:Property and implicit (subPropertyOf/domain/range)
        // properties, so object + data need not sum to `properties` — but
        // reporting the real datatype-property count is more honest than the
        // previous hardcoded 0 (which showed e.g. Schema.org / FOAF as having no
        // properties even though they declare hundreds).
        let obj_prop_query = "SELECT (COUNT(DISTINCT ?p) AS ?count) WHERE {
            ?p a <http://www.w3.org/2002/07/owl#ObjectProperty> .
            FILTER(isIRI(?p)
                && !STRSTARTS(STR(?p), \"http://www.w3.org/1999/02/22-rdf-syntax-ns#\")
                && !STRSTARTS(STR(?p), \"http://www.w3.org/2000/01/rdf-schema#\")
                && !STRSTARTS(STR(?p), \"http://www.w3.org/2002/07/owl#\"))
        }";
        let data_prop_query = "SELECT (COUNT(DISTINCT ?p) AS ?count) WHERE {
            ?p a <http://www.w3.org/2002/07/owl#DatatypeProperty> .
            FILTER(isIRI(?p)
                && !STRSTARTS(STR(?p), \"http://www.w3.org/1999/02/22-rdf-syntax-ns#\")
                && !STRSTARTS(STR(?p), \"http://www.w3.org/2000/01/rdf-schema#\")
                && !STRSTARTS(STR(?p), \"http://www.w3.org/2002/07/owl#\"))
        }";

        let classes = count_from_query(class_query);
        let props = count_from_query(prop_query);
        let object_props = count_from_query(obj_prop_query);
        let data_props = count_from_query(data_prop_query);
        let individuals = count_from_query(individual_query);

        Ok(serde_json::json!({
            "triples": total,
            "classes": classes,
            "object_properties": object_props,
            "data_properties": data_props,
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
        self.load_lines(content, RdfFormat::NTriples)
    }

    /// Load N-Quads, keeping every graph name.
    ///
    /// The line-based sibling of [`load_ntriples`](Self::load_ntriples), for
    /// the paths that round-trip a whole dataset rather than one graph — the
    /// compile cache above all, where N-Triples silently flattened everything
    /// it was asked to hold.
    pub fn load_nquads(&self, content: &str) -> anyhow::Result<usize> {
        self.load_lines(content, RdfFormat::NQuads)
    }

    fn load_lines(&self, content: &str, format: RdfFormat) -> anyhow::Result<usize> {
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

    /// Run a SPARQL query against an open (unauthenticated) endpoint.
    pub async fn fetch_sparql(endpoint: &str, query: &str) -> anyhow::Result<String> {
        Self::fetch_sparql_auth(endpoint, query, &SparqlAuth::default()).await
    }

    /// Run a SPARQL query against an endpoint, with optional HTTP auth.
    ///
    /// Works against any SPARQL 1.1 Protocol endpoint: Apache Jena/Fuseki and
    /// Eclipse RDF4J (no auth), Stardog and Ontotext GraphDB (Basic/Bearer).
    /// Amazon Neptune with IAM auth requires SigV4 request signing, which this
    /// path does not perform; use an unsigned/IAM-disabled endpoint or a signing
    /// proxy in front of Neptune.
    pub async fn fetch_sparql_auth(
        endpoint: &str,
        query: &str,
        auth: &SparqlAuth,
    ) -> anyhow::Result<String> {
        let client = reqwest::Client::new();
        let rb = client
            .post(endpoint)
            .header("Content-Type", "application/sparql-query")
            .header("Accept", "text/turtle")
            .body(query.to_string());
        let resp = auth.apply(rb).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("SPARQL endpoint returned HTTP {}", resp.status());
        }
        Ok(resp.text().await?)
    }

    /// Push triples to an open (unauthenticated) endpoint, default graph.
    pub async fn push_sparql(endpoint: &str, content: &str) -> anyhow::Result<String> {
        Self::push_sparql_auth(endpoint, content, None, &SparqlAuth::default()).await
    }

    /// Push triples to an endpoint via SPARQL 1.1 Update, with optional named
    /// graph and HTTP auth.
    pub async fn push_sparql_auth(
        endpoint: &str,
        content: &str,
        graph: Option<&str>,
        auth: &SparqlAuth,
    ) -> anyhow::Result<String> {
        let update = match graph {
            Some(g) => format!("INSERT DATA {{ GRAPH <{g}> {{ {content} }} }}"),
            None => format!("INSERT DATA {{ {content} }}"),
        };
        let client = reqwest::Client::new();
        let rb = client
            .post(endpoint)
            .header("Content-Type", "application/sparql-update")
            .body(update);
        let resp = auth.apply(rb).send().await?;
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
        } else if path.ends_with(".jsonld") || path.ends_with(".json") {
            RdfFormat::JsonLd {
                profile: JsonLdProfileSet::empty(),
            }
        } else {
            RdfFormat::Turtle
        }
    }

    /// Format detection that consults the file body, not just the extension.
    ///
    /// `.owl` is ambiguous in the wild: the extension says "an OWL ontology"
    /// and says nothing about the serialisation. Both RDF/XML and Turtle are
    /// routinely published as `.owl`, so trusting the extension alone makes a
    /// perfectly valid file fail to parse. Sniff the first non-blank,
    /// non-comment line and let the content decide.
    fn detect_format_sniffed(path: &str, content: &str) -> RdfFormat {
        let ext_format = Self::detect_format(path);

        let head = content
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with('#'))
            .unwrap_or("");

        // XML declaration or an opening tag means RDF/XML regardless of name.
        if head.starts_with("<?xml") || head.starts_with("<rdf:") || head.starts_with("<RDF") {
            return RdfFormat::RdfXml;
        }

        // Turtle/TriG directives. `<` alone is not a signal: it also opens an
        // N-Triples subject IRI, so only treat explicit directives as proof.
        let is_turtle_directive = head.starts_with("@prefix")
            || head.starts_with("@base")
            || head.to_uppercase().starts_with("PREFIX ")
            || head.to_uppercase().starts_with("BASE ");

        if is_turtle_directive && matches!(ext_format, RdfFormat::RdfXml) {
            return RdfFormat::Turtle;
        }

        // A JSON body is proof of JSON-LD in a way `{` alone is not: TriG also
        // opens its default graph block with `{`, and Turtle admits `[` as a
        // blank-node subject. Requiring a JSON-LD keyword alongside the opening
        // brace keeps those two out while still rescuing the common case of a
        // JSON-LD document published under `.owl`, `.rdf` or no extension at
        // all, which would otherwise reach the Turtle parser and die there.
        let opens_json = head.starts_with('{') || head.starts_with('[');
        let has_jsonld_keyword = content.contains("\"@context\"")
            || content.contains("\"@id\"")
            || content.contains("\"@graph\"");
        if opens_json && has_jsonld_keyword {
            return RdfFormat::JsonLd {
                profile: JsonLdProfileSet::empty(),
            };
        }

        ext_format
    }

    fn parse_format(name: &str) -> anyhow::Result<RdfFormat> {
        match name.to_lowercase().as_str() {
            "turtle" | "ttl" => Ok(RdfFormat::Turtle),
            "ntriples" | "nt" => Ok(RdfFormat::NTriples),
            "rdfxml" | "rdf" | "xml" | "owl" => Ok(RdfFormat::RdfXml),
            "nquads" | "nq" => Ok(RdfFormat::NQuads),
            "trig" => Ok(RdfFormat::TriG),
            // "json-ld" is the spelling in the W3C media type registration and
            // in most other tooling, so rejecting it turns a correct format
            // name into an error.
            "jsonld" | "json-ld" | "json" => Ok(RdfFormat::JsonLd {
                profile: JsonLdProfileSet::empty(),
            }),
            _ => anyhow::bail!(
                "Unknown format: {}. Supported: turtle, ntriples, rdfxml, nquads, trig, jsonld",
                name
            ),
        }
    }
}
