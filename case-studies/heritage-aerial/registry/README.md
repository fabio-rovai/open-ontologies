# NAPH Compliance Registry

A public registry of NAPH-compliant aerial photography heritage collections. Institutions submit declarations of compliance; the registry provides:

- A canonical list of NAPH-compliant collections sector-wide
- Tier-distribution metrics for adoption tracking
- Cross-collection discovery (which institution holds what)
- A starting point for federated SPARQL queries

## How to submit a compliance declaration

1. Generate a declaration file using the [template](compliance-declaration-template.ttl)
2. Validate it against the [registry shapes](registry-shapes.ttl)
3. Open a pull request against this directory adding your declaration to `submissions/`
4. Maintainers review and merge within 14 days
5. Your collection appears in the [public registry](https://w3id.org/naph/registry)

## What the registry stores

For each NAPH-compliant collection, the registry holds:

- Institution name + contact
- Collection name + brief description
- Tier distribution (Baseline / Enhanced / Aspirational counts)
- SPARQL endpoint URL (if available)
- Bulk-download URL
- Validation report URL (date-stamped)
- Submission and last-updated dates

## Why a registry

Without a registry, NAPH-compliant collections are hard to discover. Researchers don't know which institutions to query. Aggregators don't know what to harvest. The registry solves this with one canonical list.

## Privacy and what's NOT collected

The registry does NOT collect:

- Personal data about institutional staff beyond a single point-of-contact email
- Detailed catalogue contents (the registry points TO collections, doesn't replicate them)
- Internal operational metrics (only public-facing compliance status)

## Cross-references

- [Compliance declaration template](compliance-declaration-template.ttl)
- [Registry shapes](registry-shapes.ttl)
- [Submission process](submission-process.md)
