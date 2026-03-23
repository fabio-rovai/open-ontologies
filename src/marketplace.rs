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

/// Curated catalogue of 32 standard W3C/ISO/industry ontologies.
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
        description: "UK NDTP core ontology for information exchange — 513 classes, 206 properties, 4D extensionalist (BORO) patterns for entities, events, states, and relationships",
        domain: "upper-ontology",
        url: "https://raw.githubusercontent.com/IES-Org/ont-ies/main/docs/specification/ies-common.ttl",
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
