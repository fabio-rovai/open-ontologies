# FAQ for Collections Staff

Quick answers to common questions. Where deeper detail is needed, follow the cross-references.

## General

### What is NAPH?

NAPH is a **focused vertical digitisation standard** for aerial photography heritage collections. It specifies what metadata an aerial photograph must expose to be **computation-ready** — so that researchers, aggregators, and AI tools can use it without manual interpretation.

It's narrow on purpose. Aerial photography has distinctive features (stereo pairs, GSD, declassification provenance) that benefit from focused treatment. See [ADR-0001](../06-knowledge-transfer/architecture-decision-records/0001-narrow-vertical.md).

### Is NAPH replacing my catalogue?

**No.** NAPH is a publication layer alongside your existing catalogue. You keep your existing system (KE EMu, AdLib, ArchivesSpace, custom). You publish NAPH-compliant RDF derived from it.

### What does "computation-ready" mean?

A computational research workflow can query, filter, and aggregate the data via standard tools (SPARQL, IIIF, etc.) without manual interpretation. The opposite is "digitised but not computable" — images on the web that humans can browse but machines can't analyse at scale.

### Who's behind NAPH?

NAPH was developed by Kampakis and Co Ltd, trading as The Tesseract Academy, as a focused vertical contribution to UK aerial photography heritage research infrastructure. It is published as open source under CC BY 4.0 / MIT.

Future stewardship is proposed to be Historic Environment Scotland (HES), via the [governance proposal](../05-governance/governance-proposal.md). Standards Council representation is open to adopting institutions.

## Adoption decisions

### How long does NAPH adoption take?

For a 100,000-record aerial photography collection:

- **Baseline tier**: 4-12 weeks
- **Enhanced tier**: additional 3-6 months
- **Aspirational tier**: additional 6-12 months

These are realistic estimates assuming existing capacity. Smaller collections, or institutions with experienced linked-data staff, may go faster.

### What does it cost?

For a 100,000-record collection (notional figures):

- **Baseline**: ~£21,000 one-time
- **Enhanced** (delta): ~£110,000 one-time
- **Aspirational** (delta): ~£75,000 one-time
- Ongoing: ~£10,000-£30,000/year depending on tier

See [`docs/cost-effort-analysis.md`](../../docs/cost-effort-analysis.md) for full breakdown.

### Should we start at Aspirational?

**No.** Start at Baseline. It's incrementally upgradeable to higher tiers without rework. Most institutions should reach Baseline first across the whole collection, then upgrade high-research-value subsets to Enhanced/Aspirational selectively.

### Which records should we publish first?

Pick a high-research-value subset:

- Heavily-cited subset (records that already attract research enquiries)
- Recently digitised subset (data quality is best)
- Subsets with clearest rights status (NARA-public-domain, Crown Copyright Expired)
- Subsets with active research collaborations (researchers can validate)

Avoid starting with subsets that have known data-quality problems — those become a sink for time.

## Technical

### Do I need to learn RDF / SPARQL?

For Baseline adoption: mostly no. The transformation happens in tooling. You specify mappings; the pipeline produces RDF.

For Enhanced/Aspirational: someone on your team should understand RDF basics. The [tutorials](tutorials/) cover this in 4 hours total.

### What is SHACL?

SHACL is a W3C standard for validating RDF data. NAPH's [`ontology/naph-shapes.ttl`](../../ontology/naph-shapes.ttl) contains shapes that automatically check whether each record meets tier requirements.

In practice: you run `open-ontologies shacl naph-shapes.ttl my-data.ttl` and it tells you what's missing. No SHACL-writing required.

### Can NAPH-compliant data go in [our existing system]?

Most institutional systems can store and serve RDF. NAPH-compliant Turtle is a publication artefact — you can:

- Store it in a triple store (Apache Jena Fuseki, GraphDB, Stardog, Virtuoso)
- Serve it via your existing CMS with content negotiation
- Bundle it in BagIt for archival packaging
- Push it to aggregators (Europeana, DPLA)

Your existing catalogue keeps doing what it does. NAPH is the publication layer.

### Do I need a SPARQL endpoint?

For Baseline: no. You just need to make the records discoverable (a sitemap, a manifest, content-negotiable URIs).

For Enhanced/Aspirational: a SPARQL endpoint is recommended. Federation queries (cross-institutional) only work with SPARQL endpoints.

Hosted SPARQL services exist if you don't want to run your own — see the [skills map](../03-cost-capacity-skills/skills-map.md).

### What about IIIF?

IIIF integration is encouraged but not required at Baseline. NAPH provides a [bridge](../../pipeline/iiif-bridge.py) that generates IIIF Presentation 3.0 manifests from NAPH-compliant records.

For images to actually load in IIIF viewers (Mirador, Universal Viewer), your institution needs to run a IIIF Image API server (Cantaloupe, IIPImage). The bridge generates the manifest structure; the institution provides the image serving.

## Records and metadata

### What if our identifiers aren't URIs?

Wrap your existing internal identifiers in your namespace. If you don't have a namespace, use [w3id.org](https://w3id.org/) — it's free and permanent. See the [Identifier Policy decision tree](decision-trees/identifier-policy.md).

### What if our dates are messy?

The [date normalisation decision tree](decision-trees/date-normalisation.md) covers the common cases. The [ingest pipeline](../../pipeline/ingest.py) handles 8 common date formats out of the box.

For partial / approximate dates ("c. 1944", "March 1944"), use `xsd:gYearMonth` or `xsd:gYear` with an uncertainty annotation.

### What if we don't know the precise location?

For sortie-level coverage where individual frames have approximate location:

- Use the sortie's overall coverage as the photograph's footprint
- Note the approximation: `rdfs:comment "Sortie-level footprint; exact frame coverage not determined"`
- Plan to refine when resources allow

A coarse footprint is better than no footprint — it allows spatial queries to find the record.

### What about rights for declassified material?

Declassified material follows ordinary copyright rules from the declassification date. UK Crown Copyright on declassified WW2 material has typically expired (50 years post-publication for published works, 125 years post-creation for unpublished).

The [rights decision tree](decision-trees/rights-decision-tree.md) walks through the common cases. The [reconnaissance sub-profile](../01-standard/profiles/aerial-subprofiles/reconnaissance.md) has reconnaissance-specific notes.

### How do we handle restricted records?

Records with current restrictions (still classified, donor restrictions, in copyright with no licence):

- Can still be NAPH-compliant — the rights statement makes the restriction explicit
- The identifier still resolves but returns metadata + access notice rather than image
- The metadata is queryable; access to image content is gated

Restricted records remain part of the standard. Restriction is a fact about the record, not a reason to exclude it.

### What about indigenous / community-controlled material?

Use [Local Contexts TK Labels](https://localcontexts.org/labels/) alongside the legal rights statement. NAPH's `naph:culturalRights` property accommodates these. See [Module C](../01-standard/modules/C-rights-licensing-ethics.md).

This is part of CARE alignment.

## Adoption process

### What is a Partner Clinic?

A structured 4-step engagement (~12 hours over 4-6 weeks) where the Editorial team and an institution work together to apply NAPH to a sample of the institution's collection. Outputs: applied records, refinement findings, cost evidence, case study.

See [Partner Clinic Playbook](../05-governance/partner-clinic-playbook.md).

### Who should be involved at our institution?

For a partner clinic:

- **Sponsor** (senior, authorising engagement) — minimal time
- **Collections manager** — content expertise, scoping participation
- **Digital officer** — primary technical engagement, ~8-12 hours
- **ML/data engineer** (optional) — if Aspirational-tier work is in scope

Total institutional time commitment: 12-20 hours over 4-6 weeks.

### What about our IT department?

For Baseline tier: IT involvement is minimal — the work happens at the catalogue/data export layer.

For Enhanced/Aspirational: more involvement needed for SPARQL endpoint hosting, IIIF Image API hosting, and ongoing CI/CD.

Engage IT early if you're targeting higher tiers; the work is more pleasant when IT is a partner from the start.

## Compatibility

### Does NAPH work with [other standard]?

NAPH is built on top of W3C/OGC/IIIF standards via subclass alignment. So yes, it's compatible with:

- **DCAT** — every NAPH `Collection` is also a `dcat:Catalog`; every `AerialPhotograph` is a `dcat:Resource`
- **PROV-O** — every `CaptureEvent` and `DigitisationEvent` is a `prov:Activity`
- **SKOS** — every `Place` and `HistoricEvent` is a `skos:Concept`
- **GeoSPARQL** — every `GeographicFootprint` is a `geo:Geometry`
- **IIIF Presentation 3.0** — manifests can be generated for every photograph
- **Dublin Core** — `dcterms:type` typing aligns with DCMI vocabulary

### What about ISAD-G?

ISAD-G is the dominant archival cataloguing standard. NAPH augments rather than replaces it. The mapping between ISAD-G elements and NAPH properties is documented for archival contexts.

For aerial photography specifically, the institutional structure (Fonds → Series → File → Item) often maps to (Collection → Sortie → Frame), but NAPH doesn't enforce ISAD-G's hierarchical structure on aerial collections.

### What if we already published to Europeana?

Excellent — the work is largely transferable. Europeana ingests Europeana Data Model (EDM); NAPH-compliant collections can also be expressed in EDM via property mapping. NAPH doesn't replace EDM publication; it adds computational-research richness alongside.

### What if we already use OAI-PMH?

OAI-PMH harvesting is supported (see [Module D.2.3](../01-standard/modules/D-packaging-publication.md)). For Aspirational tier, OAI-PMH is one of the recommended publication mechanisms.

## Governance

### Who decides what changes to the standard?

The Steward (HES, post-v1.0) makes operational decisions. Substantive changes follow the [RFC process](../05-governance/rfc-process.md) with Standards Council advisory recommendation.

### Can my institution be on the Standards Council?

Yes — institutional representatives are recruited from active NAPH-adopting institutions via open call. See the [Standards Council Charter Template](../07-templates/standards-council-charter-template.md).

### What if NAPH stops being maintained?

NAPH is open-source with permanent w3id.org URIs. The repository would remain accessible; existing NAPH-compliant data would continue to validate. A community fork is possible if maintenance ever fails.

This is a low-probability scenario but the standard is designed to survive it.

## Trouble-shooting

### My SHACL validation keeps failing

Run the [self-assessment tool](../../pipeline/self-assessment.py) — it summarises violations by frequency and tells you which patterns are most common in your data.

Most common causes:

- Free-text dates (use `xsd:date` or `xsd:gYearMonth`)
- Wrong WKT coordinate order (longitude first)
- Missing `dcterms:type`
- Polygons that don't close (first/last coordinates must match)

### Spatial queries return no results

Are you using Oxigraph (the engine in Open Ontologies)? GeoSPARQL `sfIntersects` doesn't work in Oxigraph. Use Apache Jena Fuseki, GraphDB, or Stardog for geospatial queries.

Or use bounding-box numeric comparison as a fallback (works in any SPARQL engine).

### IIIF manifests don't show images in viewer

The bridge generates structurally valid manifests but the IIIF Image API URLs are placeholders. You need to run a real IIIF Image API server (Cantaloupe, IIPImage) and update the URLs in the manifest to point to it.

## Where to get help

- **Issue tracker:** [github.com/fabio-rovai/open-ontologies/issues](https://github.com/fabio-rovai/open-ontologies/issues)
- **Email:** `fabio@thetesseractacademy.com`
- **Documentation:** [`deliverables/`](../) directory
- **Tutorials:** [`tutorials/`](tutorials/)
