use oxigraph::io::RdfFormat;

/// A standard ontology available in the marketplace catalogue.
pub struct MarketplaceEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub domain: &'static str,
    pub url: &'static str,
    pub format: RdfFormat,
}

/// Curated catalogue of 33 standard W3C/ISO/industry ontologies.
pub static CATALOGUE: &[MarketplaceEntry] = &[
    // ── Foundational ──────────────────────────────────────────────
    MarketplaceEntry {
        id: "owl",
        name: "OWL 2",
        description: "W3C OWL 2 vocabulary for building ontologies",
        domain: "foundational",
        url: "https://www.w3.org/2002/07/owl#",
        format: RdfFormat::Turtle,
    },
    MarketplaceEntry {
        id: "rdfs",
        name: "RDF Schema",
        description: "W3C vocabulary for describing RDF vocabularies with classes and properties",
        domain: "foundational",
        url: "https://www.w3.org/2000/01/rdf-schema#",
        format: RdfFormat::Turtle,
    },
    MarketplaceEntry {
        id: "rdf",
        name: "RDF Concepts",
        description: "Core RDF vocabulary defining fundamental data model constructs",
        domain: "foundational",
        url: "https://www.w3.org/1999/02/22-rdf-syntax-ns",
        format: RdfFormat::Turtle,
    },

    // ── Upper ontology / Information Exchange ─────────────────────
    MarketplaceEntry {
        id: "ies-top",
        name: "IES Top Level Ontology (ToLO)",
        description: "BORO foundational ontology — extensional 4-dimensionalism and pluralities, the upper layer of the IES framework",
        domain: "upper-ontology",
        url: "https://raw.githubusercontent.com/IES-Org/ies-top/main/spec/ies-top.ttl",
        format: RdfFormat::Turtle,
    },
    MarketplaceEntry {
        id: "ies-core",
        name: "IES Core Ontology",
        description: "Core IES patterns — persons, states, events, identifiers, periods. The middle layer of the IES framework",
        domain: "upper-ontology",
        url: "https://raw.githubusercontent.com/IES-Org/ies-core/main/spec/ies-core.ttl",
        format: RdfFormat::Turtle,
    },
    MarketplaceEntry {
        id: "ies",
        name: "IES Common (Information Exchange Standard)",
        description: "UK NDTP core ontology for information exchange — 511 classes, 206 properties, 4D extensionalist (BORO) patterns for entities, events, states, and relationships",
        domain: "upper-ontology",
        url: "https://raw.githubusercontent.com/IES-Org/ont-ies/main/docs/specification/ies-common.ttl",
        format: RdfFormat::Turtle,
    },
    MarketplaceEntry {
        id: "ies-4.3.1",
        name: "IES4 v4.3.1 (frozen MIT baseline)",
        description: "Last public MIT-licensed snapshot of IES4 from the archived dstl/IES4 repo (tag v4.3.1, released 3 Mar 2025). Use as a reproducible compliance baseline when you need a frozen reference that won't shift with upstream changes. 4.3.2+ development continues in the IES-Org working group — use the `ies` preset for live work.",
        domain: "upper-ontology",
        url: "https://raw.githubusercontent.com/dstl/IES4/v4.3.1/IES%20Specification%20Docs/ies4.ttl",
        format: RdfFormat::Turtle,
    },
    MarketplaceEntry {
        id: "bfo",
        name: "BFO (Basic Formal Ontology)",
        description: "ISO 21838 upper-level ontology — foundational categories for continuants and occurrents",
        domain: "upper-ontology",
        url: "https://raw.githubusercontent.com/BFO-ontology/BFO/v2019-08-26/bfo_classes_only.owl",
        format: RdfFormat::RdfXml,
    },
    MarketplaceEntry {
        id: "dolce",
        name: "DOLCE/DUL (Descriptive Ontology)",
        description: "Upper-level ontology providing foundational categories for knowledge representation",
        domain: "upper-ontology",
        url: "http://www.ontologydesignpatterns.org/ont/dul/DUL.owl",
        format: RdfFormat::Turtle,
    },

    // ── General ───────────────────────────────────────────────────
    MarketplaceEntry {
        id: "schema-org",
        name: "Schema.org",
        description: "Collaborative vocabulary for structured data markup on the web",
        domain: "general",
        url: "https://schema.org/version/latest/schemaorg-current-https.ttl",
        format: RdfFormat::Turtle,
    },
    MarketplaceEntry {
        id: "foaf",
        name: "FOAF (Friend of a Friend)",
        description: "Vocabulary for describing people, activities, and relationships",
        domain: "people",
        url: "http://xmlns.com/foaf/spec/index.rdf",
        format: RdfFormat::RdfXml,
    },
    MarketplaceEntry {
        id: "skos",
        name: "SKOS (Simple Knowledge Organization System)",
        description: "W3C vocabulary for thesauri, classification schemes, and taxonomies",
        domain: "knowledge-organization",
        url: "https://www.w3.org/2009/08/skos-reference/skos.rdf",
        format: RdfFormat::RdfXml,
    },

    // ── Metadata ──────────────────────────────────────────────────
    MarketplaceEntry {
        id: "dc-elements",
        name: "Dublin Core Elements",
        description: "15 core metadata elements for describing resources",
        domain: "metadata",
        url: "http://www.dublincore.org/specifications/dublin-core/dcmi-terms/dublin_core_elements.ttl",
        format: RdfFormat::Turtle,
    },
    MarketplaceEntry {
        id: "dc-terms",
        name: "Dublin Core Terms",
        description: "Extended Dublin Core metadata terms with refined properties",
        domain: "metadata",
        url: "https://www.dublincore.org/specifications/dublin-core/dcmi-terms/dublin_core_terms.ttl",
        format: RdfFormat::Turtle,
    },
    MarketplaceEntry {
        id: "dcat",
        name: "DCAT (Data Catalog Vocabulary)",
        description: "W3C vocabulary for interoperability between data catalogs",
        domain: "data-catalogs",
        url: "https://www.w3.org/ns/dcat.ttl",
        format: RdfFormat::Turtle,
    },
    MarketplaceEntry {
        id: "void",
        name: "VoID (Vocabulary of Interlinked Datasets)",
        description: "Vocabulary for expressing metadata about RDF datasets",
        domain: "data-catalogs",
        url: "https://raw.githubusercontent.com/cygri/void/master/rdfs/void.ttl",
        format: RdfFormat::Turtle,
    },
    MarketplaceEntry {
        id: "doap",
        name: "DOAP (Description of a Project)",
        description: "Vocabulary for describing software projects, repositories, and releases",
        domain: "software",
        url: "https://raw.githubusercontent.com/ewilderj/doap/master/schema/doap.rdf",
        format: RdfFormat::RdfXml,
    },

    // ── Provenance ────────────────────────────────────────────────
    MarketplaceEntry {
        id: "prov-o",
        name: "PROV-O (Provenance Ontology)",
        description: "W3C ontology for representing provenance — entities, activities, agents",
        domain: "provenance",
        url: "https://www.w3.org/ns/prov-o.ttl",
        format: RdfFormat::Turtle,
    },

    // ── Temporal ──────────────────────────────────────────────────
    MarketplaceEntry {
        id: "owl-time",
        name: "OWL-Time",
        description: "W3C/OGC ontology for temporal concepts — instants, intervals, durations",
        domain: "temporal",
        url: "https://www.w3.org/2006/time.ttl",
        format: RdfFormat::Turtle,
    },

    // ── Organizations ─────────────────────────────────────────────
    MarketplaceEntry {
        id: "org",
        name: "W3C Organization Ontology",
        description: "Vocabulary for organizational structures, membership, roles, and sites",
        domain: "organizations",
        url: "https://www.w3.org/ns/org.ttl",
        format: RdfFormat::Turtle,
    },

    // ── IoT / Sensors ─────────────────────────────────────────────
    MarketplaceEntry {
        id: "ssn",
        name: "SSN (Semantic Sensor Network)",
        description: "W3C/OGC ontology for sensors, actuators, observations, and sampling",
        domain: "iot",
        url: "https://raw.githubusercontent.com/w3c/sdw-sosa-ssn/gh-pages/ssn/rdf/ontology/core/ssn.ttl",
        format: RdfFormat::Turtle,
    },
    MarketplaceEntry {
        id: "sosa",
        name: "SOSA (Sensor, Observation, Sample, Actuator)",
        description: "Lightweight core of SSN for sensors and observations",
        domain: "iot",
        url: "https://raw.githubusercontent.com/w3c/sdw-sosa-ssn/gh-pages/ssn/rdf/ontology/core/sosa.ttl",
        format: RdfFormat::Turtle,
    },

    // ── Geospatial ────────────────────────────────────────────────
    MarketplaceEntry {
        id: "geosparql",
        name: "GeoSPARQL",
        description: "OGC ontology for spatial objects, geometries, and topological relations",
        domain: "geospatial",
        url: "https://opengeospatial.github.io/ogc-geosparql/geosparql11/geo.ttl",
        format: RdfFormat::Turtle,
    },
    MarketplaceEntry {
        id: "locn",
        name: "LOCN (Location Core Vocabulary)",
        description: "EU ISA vocabulary for describing places by name, address, or geometry",
        domain: "geospatial",
        url: "https://www.w3.org/ns/locn.ttl",
        format: RdfFormat::Turtle,
    },

    // ── Validation ────────────────────────────────────────────────
    MarketplaceEntry {
        id: "shacl",
        name: "SHACL (Shapes Constraint Language)",
        description: "W3C vocabulary for validating RDF graphs against shapes and constraints",
        domain: "validation",
        url: "https://www.w3.org/ns/shacl.ttl",
        format: RdfFormat::Turtle,
    },

    // ── People / Contact ──────────────────────────────────────────
    MarketplaceEntry {
        id: "vcard",
        name: "vCard Ontology",
        description: "Ontology for representing contact information in RDF",
        domain: "people",
        url: "http://www.w3.org/2006/vcard/ns",
        format: RdfFormat::Turtle,
    },

    // ── Rights / Licensing ────────────────────────────────────────
    MarketplaceEntry {
        id: "odrl",
        name: "ODRL (Open Digital Rights Language)",
        description: "W3C vocabulary for expressing permissions, prohibitions, and obligations",
        domain: "rights",
        url: "https://www.w3.org/ns/odrl/2/ODRL22.ttl",
        format: RdfFormat::Turtle,
    },
    MarketplaceEntry {
        id: "cc",
        name: "Creative Commons",
        description: "Vocabulary for describing copyright licenses and permissions",
        domain: "rights",
        url: "https://creativecommons.org/schema.rdf",
        format: RdfFormat::RdfXml,
    },

    // ── Social ────────────────────────────────────────────────────
    MarketplaceEntry {
        id: "sioc",
        name: "SIOC (Semantically-Interlinked Online Communities)",
        description: "Ontology for describing online communities, forums, and posts",
        domain: "social",
        url: "https://raw.githubusercontent.com/VisualDataWeb/OWL2VOWL/master/ontologies/sioc.rdf",
        format: RdfFormat::RdfXml,
    },

    // ── E-Government ──────────────────────────────────────────────
    MarketplaceEntry {
        id: "adms",
        name: "ADMS (Asset Description Metadata Schema)",
        description: "EU ISA vocabulary for describing semantic assets and interoperability solutions",
        domain: "egovernment",
        url: "https://www.w3.org/ns/adms.ttl",
        format: RdfFormat::Turtle,
    },

    // ── Commerce ──────────────────────────────────────────────────
    MarketplaceEntry {
        id: "goodrelations",
        name: "GoodRelations",
        description: "Ontology for e-commerce — products, services, prices, and offers",
        domain: "commerce",
        url: "http://www.heppnetz.de/ontologies/goodrelations/v1.owl",
        format: RdfFormat::RdfXml,
    },

    // ── Finance ───────────────────────────────────────────────────
    MarketplaceEntry {
        id: "fibo",
        name: "FIBO (Financial Industry Business Ontology)",
        description: "EDM Council ontology for financial industry concepts",
        domain: "finance",
        url: "https://spec.edmcouncil.org/fibo/ontology/master/latest/MetadataFIBO.rdf",
        format: RdfFormat::RdfXml,
    },

    // ── Science / Measurement ─────────────────────────────────────
    MarketplaceEntry {
        id: "qudt",
        name: "QUDT (Quantities, Units, Dimensions, Types)",
        description: "Ontology for physical quantities, units of measure, and dimensions",
        domain: "science",
        url: "http://qudt.org/2.1/schema/qudt",
        format: RdfFormat::Turtle,
    },
];

/// Look up a marketplace entry by ID.
pub fn find(id: &str) -> Option<&'static MarketplaceEntry> {
    CATALOGUE.iter().find(|e| e.id == id)
}

/// List all entries, optionally filtered by domain.
pub fn list(domain: Option<&str>) -> Vec<&'static MarketplaceEntry> {
    match domain {
        Some(d) => CATALOGUE.iter().filter(|e| e.domain == d).collect(),
        None => CATALOGUE.iter().collect(),
    }
}

/// Format name for the RDF format.
pub fn format_name(fmt: RdfFormat) -> &'static str {
    match fmt {
        RdfFormat::Turtle => "turtle",
        RdfFormat::RdfXml => "rdfxml",
        RdfFormat::NTriples => "ntriples",
        RdfFormat::NQuads => "nquads",
        RdfFormat::TriG => "trig",
        _ => "unknown",
    }
}

/// Parse a manifest format string back into an RdfFormat.
pub fn parse_format(s: &str) -> Option<RdfFormat> {
    match s {
        "turtle" => Some(RdfFormat::Turtle),
        "rdfxml" => Some(RdfFormat::RdfXml),
        "ntriples" => Some(RdfFormat::NTriples),
        "nquads" => Some(RdfFormat::NQuads),
        "trig" => Some(RdfFormat::TriG),
        _ => None,
    }
}

// ─── Community packs ─────────────────────────────────────────────────────────
//
// The curated CATALOGUE above is compiled in and vetted by maintainers. The
// community registry is the open-submission tier: a JSON file of pack
// manifests, contributed by PR to `community/registry.json` and fetched at
// runtime, so new packs become installable without a release. Packs are DATA
// (ontology files fetched over HTTP), never code.

/// A community-contributed ontology pack, loaded at runtime from the registry.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CommunityEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub domain: String,
    pub url: String,
    /// One of: turtle, rdfxml, ntriples, nquads, trig
    pub format: String,
    #[serde(default)]
    pub maintainer: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CommunityRegistry {
    pub version: u32,
    pub packs: Vec<CommunityEntry>,
}

/// Canonical location of the community registry on the default branch.
pub const DEFAULT_COMMUNITY_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/fabio-rovai/open-ontologies/main/community/registry.json";

/// Resolve where the community registry comes from, in priority order:
/// 1. `OPEN_ONTOLOGIES_COMMUNITY_REGISTRY` (a URL or a local file path)
/// 2. `./community/registry.json` if present (source checkouts, air-gapped installs)
/// 3. the canonical GitHub raw URL
pub fn community_registry_source() -> String {
    if let Ok(v) = std::env::var("OPEN_ONTOLOGIES_COMMUNITY_REGISTRY")
        && !v.trim().is_empty() {
            return v;
        }
    let local = std::path::Path::new("community/registry.json");
    if local.exists() {
        return local.to_string_lossy().into_owned();
    }
    DEFAULT_COMMUNITY_REGISTRY_URL.to_string()
}

/// Fetch and parse the community registry from wherever
/// [`community_registry_source`] resolves to. Returns (packs, shadowed_ids,
/// source) so callers can attribute provenance in their output.
pub async fn load_community_packs() -> Result<(Vec<CommunityEntry>, Vec<String>, String), String> {
    let source = community_registry_source();
    let json = if source.starts_with("http://") || source.starts_with("https://") {
        crate::graph::GraphStore::fetch_url(&source)
            .await
            .map_err(|e| format!("registry fetch failed ({source}): {e}"))?
    } else {
        std::fs::read_to_string(&source)
            .map_err(|e| format!("registry read failed ({source}): {e}"))?
    };
    let (packs, shadowed) = parse_community_registry(&json)?;
    Ok((packs, shadowed, source))
}

/// Parse and validate registry JSON. Rejects malformed manifests outright;
/// packs whose IDs collide with the curated catalogue are dropped (curated
/// wins) and reported back so the caller can surface the shadowing.
pub fn parse_community_registry(json: &str) -> Result<(Vec<CommunityEntry>, Vec<String>), String> {
    let registry: CommunityRegistry =
        serde_json::from_str(json).map_err(|e| format!("invalid registry JSON: {e}"))?;
    if registry.version != 1 {
        return Err(format!("unsupported registry version {}", registry.version));
    }
    let mut shadowed = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut packs = Vec::new();
    for pack in registry.packs {
        if pack.id.is_empty()
            || !pack.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(format!(
                "pack id '{}' is invalid: ids are lowercase kebab-case ([a-z0-9-])",
                pack.id
            ));
        }
        if parse_format(&pack.format).is_none() {
            return Err(format!(
                "pack '{}' declares unknown format '{}' (expected turtle|rdfxml|ntriples|nquads|trig)",
                pack.id, pack.format
            ));
        }
        if !(pack.url.starts_with("https://") || pack.url.starts_with("http://")) {
            return Err(format!("pack '{}' url must be http(s): {}", pack.id, pack.url));
        }
        if find(&pack.id).is_some() {
            shadowed.push(pack.id);
            continue;
        }
        if !seen.insert(pack.id.clone()) {
            return Err(format!("duplicate pack id '{}' in registry", pack.id));
        }
        packs.push(pack);
    }
    Ok((packs, shadowed))
}

// ─── Shared CLI surface ──────────────────────────────────────────────
//
// `marketplace list` and `marketplace install` are one user-facing command with
// two implementations behind it: the local one in `main.rs` and the batch one in
// `batch.rs` that serves it whenever a daemon is running. The two had drifted —
// the batch copy consulted only the curated catalogue and never loaded community
// packs — so the same command answered with a different catalogue depending on
// whether a daemon happened to be up, and said nothing about the difference.
// Both now go through the functions below.
//
// The MCP tool in `server.rs` deliberately reports a richer shape (urls,
// maintainer, shadowing warnings) and is left as its own surface.

/// One pack resolved for installation, from either tier.
pub struct ResolvedPack {
    pub id: String,
    pub name: String,
    pub url: String,
    pub format: RdfFormat,
}

/// The catalogue as the CLI reports it: curated entries first, then community
/// packs, which can never shadow a curated id. The second element is a message
/// about the community registry when it could not be loaded, which is reported
/// rather than being allowed to fail the listing.
pub async fn cli_list(domain: Option<&str>) -> (Vec<serde_json::Value>, Option<String>) {
    let mut items: Vec<serde_json::Value> = list(domain)
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id,
                "name": e.name,
                "description": e.description,
                "domain": e.domain,
                "format": format_name(e.format),
                "source": "curated",
            })
        })
        .collect();

    let mut community_error = None;
    match load_community_packs().await {
        Ok((packs, _shadowed, _source)) => {
            for p in packs
                .iter()
                .filter(|p| domain.is_none_or(|d| p.domain == d))
            {
                items.push(serde_json::json!({
                    "id": p.id,
                    "name": p.name,
                    "description": p.description,
                    "domain": p.domain,
                    "format": p.format,
                    "source": "community",
                }));
            }
        }
        Err(e) => community_error = Some(e),
    }
    (items, community_error)
}

/// Resolve an install id across both tiers, curated first. The error is the
/// message to show the user.
pub async fn cli_resolve(id: &str) -> Result<ResolvedPack, String> {
    if let Some(e) = find(id) {
        return Ok(ResolvedPack {
            id: e.id.to_string(),
            name: e.name.to_string(),
            url: e.url.to_string(),
            format: e.format,
        });
    }
    let community = match load_community_packs().await {
        Ok((packs, _, _)) => packs.into_iter().find(|p| p.id == id),
        Err(_) => None,
    };
    match community.and_then(|p| {
        parse_format(&p.format).map(|f| ResolvedPack {
            id: p.id,
            name: p.name,
            url: p.url,
            format: f,
        })
    }) {
        Some(r) => Ok(r),
        None => Err(format!(
            "Unknown ontology ID: '{}'. Run 'marketplace list' to see curated and community IDs.",
            id
        )),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn find(id: &str) -> Option<&MarketplaceEntry> {
        CATALOGUE.iter().find(|e| e.id == id)
    }

    #[test]
    fn ies_4_3_1_preset_exists_and_targets_archived_dstl_v4_3_1_tag() {
        // Per #25 — `ies-4.3.1` is the frozen MIT-licensed baseline. Must
        // point at the dstl/IES4 archived repo at tag `v4.3.1`, NOT main
        // or any other branch (the whole point of the preset is reproducible
        // pinning).
        let entry = find("ies-4.3.1").expect(
            "ies-4.3.1 marketplace preset missing — was the entry removed by mistake?",
        );
        assert!(entry.url.contains("dstl/IES4"), "url should reference the archived dstl/IES4 repo; got {}", entry.url);
        assert!(entry.url.contains("/v4.3.1/"), "url MUST pin to tag v4.3.1; got {}", entry.url);
        assert!(entry.url.ends_with("ies4.ttl"), "expected Turtle artefact; got {}", entry.url);
        assert!(matches!(entry.format, RdfFormat::Turtle));
        assert_eq!(entry.domain, "upper-ontology");
    }

    #[test]
    fn ies_4_3_1_does_not_collide_with_live_ies_preset() {
        // The live `ies` preset (pointing at IES-Org main) and the frozen
        // `ies-4.3.1` preset must coexist with distinct IDs and URLs —
        // they serve different purposes.
        let live = find("ies").expect("live `ies` preset missing");
        let frozen = find("ies-4.3.1").expect("frozen `ies-4.3.1` preset missing");
        assert_ne!(live.id, frozen.id);
        assert_ne!(live.url, frozen.url);
        assert!(live.url.contains("IES-Org"), "live preset should point at IES-Org");
        assert!(frozen.url.contains("dstl/IES4"), "frozen preset should point at archived dstl/IES4");
    }

    #[test]
    fn community_registry_parses_and_validates() {
        let json = r#"{"version":1,"packs":[{
            "id":"pizza","name":"Pizza Ontology","description":"Teaching ontology",
            "domain":"teaching","url":"https://example.org/pizza.owl","format":"rdfxml",
            "maintainer":"someone","license":"CC-BY-4.0"}]}"#;
        let (packs, shadowed) = parse_community_registry(json).unwrap();
        assert_eq!(packs.len(), 1);
        assert!(shadowed.is_empty());
        assert_eq!(packs[0].id, "pizza");
        assert!(parse_format(&packs[0].format).is_some());
    }

    #[test]
    fn community_pack_shadowing_curated_id_is_dropped_and_reported() {
        // A community pack must never override a curated entry — `foaf` is curated.
        let json = r#"{"version":1,"packs":[{
            "id":"foaf","name":"Fake FOAF","description":"x",
            "domain":"people","url":"https://example.org/x.ttl","format":"turtle"}]}"#;
        let (packs, shadowed) = parse_community_registry(json).unwrap();
        assert!(packs.is_empty());
        assert_eq!(shadowed, vec!["foaf"]);
    }

    #[test]
    fn community_registry_rejects_bad_ids_formats_urls_and_dupes() {
        let bad_id = r#"{"version":1,"packs":[{"id":"Bad_ID","name":"x","description":"x","domain":"x","url":"https://e.org/x","format":"turtle"}]}"#;
        assert!(parse_community_registry(bad_id).is_err());
        let bad_format = r#"{"version":1,"packs":[{"id":"ok","name":"x","description":"x","domain":"x","url":"https://e.org/x","format":"jsonld"}]}"#;
        assert!(parse_community_registry(bad_format).is_err());
        let bad_url = r#"{"version":1,"packs":[{"id":"ok","name":"x","description":"x","domain":"x","url":"ftp://e.org/x","format":"turtle"}]}"#;
        assert!(parse_community_registry(bad_url).is_err());
        let dupe = r#"{"version":1,"packs":[
            {"id":"ok","name":"x","description":"x","domain":"x","url":"https://e.org/x","format":"turtle"},
            {"id":"ok","name":"y","description":"y","domain":"y","url":"https://e.org/y","format":"turtle"}]}"#;
        assert!(parse_community_registry(dupe).is_err());
        let bad_version = r#"{"version":2,"packs":[]}"#;
        assert!(parse_community_registry(bad_version).is_err());
    }

    #[test]
    fn shipped_community_registry_is_valid() {
        // The registry that ships in-tree (and is served from GitHub raw as the
        // default remote registry) must always parse — a broken merge here
        // would break every user's `onto_marketplace list`.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/community/registry.json");
        let json = std::fs::read_to_string(path).expect("community/registry.json missing");
        let (packs, shadowed) = parse_community_registry(&json).expect("shipped registry invalid");
        assert!(shadowed.is_empty(), "shipped registry shadows curated ids: {shadowed:?}");
        assert!(!packs.is_empty(), "shipped registry should seed at least one pack");
    }
}
