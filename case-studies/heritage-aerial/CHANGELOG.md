# Changelog

All notable changes to NAPH are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.0.0] — 2026-04-30

Initial v1.0 release of the NAPH standard for aerial photography heritage.

### Added

- NAPH ontology (`ontology/naph-core.ttl`) — 30 classes, 29 properties, 198 triples
- SHACL shapes (`ontology/naph-shapes.ttl`) — tiered Baseline / Enhanced / Aspirational + DigitalSurrogate + Place + RightsStatement
- 10-record sample dataset modelled on NCAP collection structure
- 6 module specifications (A: Capture, B: Metadata, C: Rights, D: Packaging, E: Paradata, F: QA)
- Aerial Photography Profile (the single normative profile in v1.0)
- 6 architecture decision records (ADRs)
- Adoption guidance, validation checklists, decision trees (rights, identifiers, dates)
- Cost & effort analysis with skills map and investment case
- Governance proposal with RFC process
- Maintenance runbook for HES (or successor steward)
- Partner clinic playbook (4-step engagement cycle)
- CSV → NAPH ingest pipeline (`pipeline/ingest.py`)
- IIIF Presentation 3.0 bridge (`pipeline/iiif-bridge.py`)
- Validation report generator (`pipeline/generate-report.py`)
- Self-assessment CLI (`pipeline/self-assessment.py`)
- Interactive map demo (`demo/index.html`)
- GitHub Actions CI/CD validation workflow
- Red-team report documenting v0.x → v1.0 corrections

### Decisions documented in ADRs

- ADR-0001: Narrow vertical scope (aerial photography only)
- ADR-0002: Synthesis over invention (subclass alignment to W3C/OGC)
- ADR-0003: AerialPhotograph as `dcat:Resource`, not `dcat:Dataset`
- ADR-0004: Three-tier nested compliance model
- ADR-0005: Outcome requirements over prescriptive workflows
- ADR-0006: Permanent URIs via w3id.org

### Known limitations

- GeoSPARQL `sfIntersects` queries do not work in the Oxigraph triple store backing Open Ontologies; spatial queries require Apache Jena Fuseki, GraphDB, Stardog, or equivalent
- IIIF Image Service URLs in generated manifests are placeholders; institutions adopting NAPH must run a real IIIF Image API server (Cantaloupe, IIPImage) for image serving
- Partial-date support (`xsd:gYearMonth`, `xsd:gYear`) is specified but the ingest pipeline currently only emits full dates
- Field-of-view geometric derivation for footprints is specified in the Aerial Photography Profile but not yet implemented in the ingest pipeline (planned v0.3)

### Pre-1.0 development history

Pre-1.0 development was undertaken by Kampakis and Co Ltd, trading as The Tesseract Academy, as a focused vertical contribution to UK aerial photography heritage research infrastructure. The v1.0 work product is published under CC BY 4.0 / MIT to maximise sector benefit.
