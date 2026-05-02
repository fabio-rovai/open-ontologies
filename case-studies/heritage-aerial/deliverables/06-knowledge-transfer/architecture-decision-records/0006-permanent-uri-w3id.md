# ADR-0006: Permanent URIs via w3id.org

**Status:** Accepted
**Date:** 2026-04-30
**Decider:** Editorial team

## Context

The NAPH ontology and standard need a permanent, stable, dereferenceable URI. Options:

(a) Host under the host institution's domain (e.g. `https://standards.hes.scot/naph/ontology`)
(b) Use a permanent URL service that decouples persistence from any specific website's stability

The choice has long-term consequences. NAPH must remain resolvable for decades.

## Decision

NAPH uses **w3id.org** as the canonical URI provider:

- Ontology namespace: `https://w3id.org/naph/ontology#`
- Permanent URL: redirects via [github.com/perma-id/w3id.org](https://github.com/perma-id/w3id.org) to the current canonical TTL location

## Consequences

### Positive

- **URI stability independent of any website** — w3id.org has been operating since 2014, with redundancy and community governance
- **Free** — no licensing fees, no ongoing operational cost beyond making PRs
- **Community-trusted** — used by W3C, Schema.org, Bioschemas, RO-Crate, and many other standards
- **Easy migration** — when the canonical TTL location changes, update the redirect via PR; existing users see no change
- **Multi-stakeholder maintained** — the redirect itself is governed by community PRs, not by a single institution
- **Aligned with linked-data community norms** — most widely-used linked-data standards use w3id.org or purl.org

### Negative

- **Requires ongoing engagement with w3id.org governance** — PRs to update redirects need to be submitted and reviewed. Low-effort but not zero.
- **Slight indirection** — URI resolution goes through w3id.org's redirect service, adding one HTTP hop
- **Trust dependency** — NAPH's URI persistence depends on w3id.org continuing to operate. Mitigated by the service's track record and community ownership.

### Neutral

- The w3id.org service is run by the W3C Permanent Identifier Community Group and W3C contractors. It is independent of any single institution.

## Alternatives considered

### Alternative 1: HES-domain hosting

E.g. `https://standards.hes.scot/naph/ontology`

Rejected because:

- HES website redesigns, restructures, or domain changes would break the URI
- If HES changes role (less likely but possible) the URI becomes orphaned
- Other institutions adopting NAPH may resist HES-namespace anchoring (perceived sovereignty issue)

### Alternative 2: purl.org

Considered. `purl.org` is operated by OCLC and has a long track record. It's a viable alternative to w3id.org.

Rejected (slight preference) because:

- purl.org is operated by a single (commercial-but-non-profit) institution; w3id.org is community-governed
- w3id.org has stronger linked-data community presence
- Migration cost between the two is similar — choice is not strongly load-bearing

If a future Standards Council prefers purl.org, the migration is feasible.

### Alternative 3: DOI

Rejected because:

- DOIs are designed for individual research artefacts (papers, datasets), not ontology namespaces
- DOI usage requires per-allocation cost
- DOI URIs aren't natural namespaces for ontologies

### Alternative 4: ARK (Archival Resource Key)

Considered but rejected because:

- ARK is well-suited for archival content but less standard for ontologies
- ARK community is smaller than w3id.org community for ontology use
- Not aligned with how comparable standards (PROV, DCAT, IIIF) self-identify

### Alternative 5: Cool URIs without redirect

E.g. `http://naph.standards.scot/ontology` (a domain owned by the standards body but with no redirect)

Rejected because:

- Requires the standards body to maintain DNS, certificates, hosting indefinitely
- Adds operational burden compared to w3id.org redirect
- One-step indirect (via w3id.org) is fine; direct hosting requires more

## Implementation

w3id.org configuration:

```apache
# In w3id.org/naph/.htaccess

# Default — redirect to canonical TTL
RewriteEngine On

# Redirect /naph to documentation
RewriteRule ^/?$ https://github.com/<org>/naph/blob/main/README.md [R=302,L]

# Redirect /naph/ontology to current TTL
RewriteRule ^ontology/?$ https://github.com/<org>/naph/raw/main/ontology/naph-core.ttl [R=302,L]

# Redirect specific resources via fragments
RewriteRule ^ontology#(.*) https://github.com/<org>/naph/raw/main/ontology/naph-core.ttl#$1 [R=302,L]

# Handle versioned access
RewriteRule ^ontology/v(\d+\.\d+\.\d+)$ https://github.com/<org>/naph/raw/v$1/ontology/naph-core.ttl [R=302,L]
```

The PR submission to perma-id/w3id.org must:

- Be reviewed by w3id.org maintainers (~1-7 days)
- Document the namespace's purpose and governance
- Reference a stable institutional contact for the namespace

## Operational discipline

The Steward MUST:

- Never let the redirect target return 404
- Update the redirect within 24 hours of canonical TTL location changes
- Test resolution before each release
- Maintain GitHub releases with stable tags so version-specific URIs work

## Validation

The chosen approach is validated by:

- Multiple comparable standards using w3id.org successfully (RO-Crate, IIIF, Schema.org partner standards)
- Track record of >10 years of w3id.org operation
- Low operational burden in practice — redirects rarely need updating

## Migration plan

If at some future point NAPH needs to migrate away from w3id.org:

1. Identify successor URI scheme
2. Set up redirect from successor scheme to existing canonical TTL
3. Update w3id.org redirect to point to successor scheme
4. Maintain both for at least 3 years
5. Eventually transition w3id.org redirect to successor scheme directly

This migration would be the same regardless of starting URI. There's no lock-in penalty.

## Cross-references

- [w3id.org service](https://w3id.org/)
- [perma-id/w3id.org GitHub](https://github.com/perma-id/w3id.org)
- [NAPH-STANDARD §7 — Naming and namespaces](../../01-standard/NAPH-STANDARD.md#7-naming-namespaces-and-identifiers)
- [Maintenance runbook §6 — Permanent URI maintenance](../maintenance-runbook.md#6-permanent-uri-maintenance)
