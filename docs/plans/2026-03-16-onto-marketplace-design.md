# onto_marketplace Design

## Summary

Add an `onto_marketplace` tool (tool #43) that provides a curated catalogue of 29 standard W3C/ISO/industry ontologies, fetchable with a single command. Built on top of existing `fetch_url` + `load_turtle`/`load_content` infrastructure.

## Motivation

Every real ontology project starts from industry standards, not a blank slate. Palantir sells pre-built "industry models" as a premium feature. We give away 29 standard ontologies for free -- proper OWL/RDF standards that interoperate with any tool in the semantic web ecosystem.

## Tool Interface

### Input

```rust
struct OntoMarketplaceInput {
    action: String,         // "list" or "install"
    id: Option<String>,     // e.g. "prov-o", "schema-org", "foaf"
    domain: Option<String>, // filter list by domain
}
```

### Actions

- **list**: Returns JSON array of `{id, name, description, domain, url, format}`. Filterable by domain.
- **install**: Fetches the ontology by ID from its canonical URL, loads it into the triple store, returns triple count + class/property stats.

## Catalogue (29 ontologies)

| ID | Name | Domain | Format | URL |
|---|---|---|---|---|
| owl | OWL 2 | foundational | turtle | https://www.w3.org/2002/07/owl# |
| rdfs | RDF Schema | foundational | turtle | https://www.w3.org/2000/01/rdf-schema# |
| rdf | RDF Concepts | foundational | turtle | https://www.w3.org/1999/02/22-rdf-syntax-ns |
| schema-org | Schema.org | general | turtle | https://schema.org/version/latest/schemaorg-current-https.ttl |
| dc-elements | Dublin Core Elements | metadata | turtle | http://www.dublincore.org/specifications/dublin-core/dcmi-terms/dublin_core_elements.ttl |
| dc-terms | Dublin Core Terms | metadata | turtle | https://www.dublincore.org/specifications/dublin-core/dcmi-terms/dublin_core_terms.ttl |
| foaf | FOAF | people | rdfxml | http://xmlns.com/foaf/spec/index.rdf |
| skos | SKOS | knowledge-organization | rdfxml | https://www.w3.org/2009/08/skos-reference/skos.rdf |
| prov-o | PROV-O | provenance | turtle | https://www.w3.org/ns/prov-o.ttl |
| owl-time | OWL-Time | temporal | turtle | https://www.w3.org/2006/time.ttl |
| org | W3C Organization | organizations | turtle | https://www.w3.org/ns/org.ttl |
| dcat | DCAT | data-catalogs | turtle | https://www.w3.org/ns/dcat.ttl |
| ssn | SSN | iot | turtle | https://raw.githubusercontent.com/w3c/sdw-sosa-ssn/gh-pages/ssn/rdf/ontology/core/ssn.ttl |
| sosa | SOSA | iot | turtle | https://raw.githubusercontent.com/w3c/sdw-sosa-ssn/gh-pages/ssn/rdf/ontology/core/sosa.ttl |
| geosparql | GeoSPARQL | geospatial | turtle | https://opengeospatial.github.io/ogc-geosparql/geosparql11/geo.ttl |
| shacl | SHACL | validation | turtle | https://www.w3.org/ns/shacl.ttl |
| vcard | vCard | people | turtle | http://www.w3.org/2006/vcard/ns |
| adms | ADMS | egovernment | turtle | https://www.w3.org/ns/adms.ttl |
| odrl | ODRL | rights | turtle | https://www.w3.org/ns/odrl/2/ODRL22.ttl |
| doap | DOAP | software | rdfxml | https://raw.githubusercontent.com/ewilderj/doap/master/schema/doap.rdf |
| sioc | SIOC | social | rdfxml | https://raw.githubusercontent.com/VisualDataWeb/OWL2VOWL/master/ontologies/sioc.rdf |
| cc | Creative Commons | rights | rdfxml | https://creativecommons.org/schema.rdf |
| void | VoID | data-catalogs | turtle | https://raw.githubusercontent.com/cygri/void/master/rdfs/void.ttl |
| goodrelations | GoodRelations | commerce | rdfxml | http://www.heppnetz.de/ontologies/goodrelations/v1.owl |
| bfo | BFO | upper-ontology | rdfxml | https://raw.githubusercontent.com/BFO-ontology/BFO/v2019-08-26/bfo_classes_only.owl |
| dolce | DOLCE/DUL | upper-ontology | rdfxml | http://www.ontologydesignpatterns.org/ont/dul/DUL.owl |
| fibo | FIBO | finance | rdfxml | https://spec.edmcouncil.org/fibo/ontology/master/latest/MetadataFIBO.rdf |
| qudt | QUDT | science | turtle | http://qudt.org/2.1/schema/qudt |
| locn | LOCN | geospatial | turtle | https://www.w3.org/ns/locn.ttl |

## Implementation

### Changes required

1. **`src/graph.rs`** -- Add `load_content(&self, content: &str, format: RdfFormat) -> Result<usize>` to handle fetched content in any RDF format (not just Turtle).

2. **`src/inputs.rs`** -- Add `OntoMarketplaceInput` struct.

3. **`src/marketplace.rs`** (new) -- Catalogue definition as a static array of `MarketplaceEntry` structs.

4. **`src/server.rs`** -- Add `onto_marketplace` tool method.

5. **`src/main.rs`** -- Add `marketplace` CLI subcommand.

### Catalogue storage

Hardcoded Rust array. No remote registry. Standard ontologies don't change often -- updates come with new releases of the binary.

### Format handling

Each catalogue entry stores its RDF format (turtle or rdfxml). The `install` action uses `fetch_url` then `load_content(content, format)` instead of `load_turtle` to handle both formats.

## Benchmarking

After implementation, benchmark all 29 ontologies through the full pipeline:
- `onto_clear` -> `onto_marketplace install <id>` -> `onto_stats` -> `onto_reason` -> `onto_stats`

Record: fetch time, triple count (before/after reasoning), class count, property count. Add results to README.
