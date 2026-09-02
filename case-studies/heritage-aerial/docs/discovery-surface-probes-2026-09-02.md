# Discovery-surface probes, 2 September 2026

All observations below were made live on 2 September 2026 with `curl -sL`
(redirects followed) from a UK-account residential connection. They are
point-in-time observations of deployed web infrastructure and can change at any
time. Each numbered line is an evidence anchor for a claim in Section 7 of the
paper.

## NCAP (United Kingdom): catalogue host airphotofinder.ncap.org

1. `https://ncap.org.uk` → HTTP/2 301, `location: https://www.ncap.org/`.
2. `https://www.ncap.org/robots.txt` → valid robots file. Declares
   `sitemap: https://www.ncap.org/sitemaps-1-sitemap.xml`. Disallows
   `/cpresources/`, `/vendor/`, `/.env`, `/cache/` for all agents, and fully
   disallows GPTBot, Google-Extended and PerplexityBot.
3. `https://www.ncap.org/sitemaps-1-sitemap.xml` → valid sitemap index; entries
   observed cover editorial sections (`caseStudies`, `guides`, `news`, ...).
   No image sitemap entries observed in the index head.
4. `https://airphotofinder.ncap.org/robots.txt` → 200 `text/html`: the Angular
   application shell, not a robots file.
5. `https://airphotofinder.ncap.org/sitemap.xml` → 200 `text/html`: the same
   application shell.
6. `https://airphotofinder.ncap.org/image/797810` → 200 `text/html`,
   50,661 bytes, `<title>NCAP Geoportal</title>`. The string `797810` occurs
   zero times in the delivered HTML.

## NAPL open data (Canada): catalogue host open.canada.ca

7. `https://open.canada.ca/robots.txt` → valid robots file (73 Disallow rules;
   none matching `/data/` or `dataset`).
8. `https://open.canada.ca/sitemap.xml` → 200 `application/xml`.
9. `https://open.canada.ca/data/en/dataset/114417c3-41a4-4cf7-8acd-2eb9d16c97f3`
   → 200, 42,069 bytes of server-rendered HTML. Title: "Collection of Temporal
   Series of the National Air Photo Library (NAPL) - Open Government Portal".
   The phrase "National Air Photo" occurs 9 times in the delivered HTML;
   `schema.org` is referenced; no `noindex` directive present.

## WHAIFinder (United States): catalogue host maps.sco.wisc.edu

10. `https://maps.sco.wisc.edu/robots.txt` → valid robots file, verbatim:
    `User-agent: *` / `Disallow: /`, then `Browsershots` (no disallow),
    `Googlebot` / `Allow: /`, `Mediapartners-Google` / `Allow: /`,
    `Googlebot-Mobile` / `Allow: /`.
11. `https://maps.sco.wisc.edu/sitemap.xml` → 404.
12. `https://maps.sco.wisc.edu/whaifinder/` → 200, 1,315 bytes: a JavaScript
    application shell with 2 script tags, no schema.org markup, and no record
    content in the delivered HTML.

## Deployed consumer of machine-readable licensing metadata

13. `https://developers.google.com/search/docs/appearance/structured-data/image-license-metadata`
    → 200, "Google Images SEO: Image Metadata", Google Search Central
    documentation, accessed 2 September 2026. States: providing licensing
    information makes an image eligible for the Licensable badge, "which
    provides a link to the license and more detail on how someone can use the
    image"; the information is supplied either as schema.org `ImageObject`
    structured data (`license`, `acquireLicensePage`) or as IPTC photo metadata
    (Web Statement of Rights, Licensor URL), either being sufficient; Google
    recommends submitting a sitemap to keep results current.
14. `https://schema.org/acquireLicensePage` → 200.

## Additional probes, same day (2 September 2026)

15. WHAIFinder FeatureServer live counts
    (`.../Wisconsin_Historic_Aerial_Imagery/FeatureServer/0/query`):
    total records 318,295; records with `map_scale_denom>0`: 106,597. Grouping
    the scale-null records by `collection_identifier` returns modern digital
    orthoimagery series (for example "Barron County 2024" 6,400, "Bayfield
    County 2025" 6,392, "Marathon County 2025" 6,336, `doq_qq` 5,184), not
    historic film.
16. `https://docs.ogc.org/cs/25-004/25-004.html` → title "SpatioTemporal Asset
    Catalog (STAC) Community Standard", Version: 1.1, dates 2025-09-09
    (approval) and 2025-10-14 (publication). STAC 1.0.0 release date
    2021-05-25 per the stac-spec releases.
17. RiC-O 1.1 source (`RiC-O_1-1.rdf`, ICA-EGAD master, parsed with rdflib):
    107 owl:Class declarations in the rico namespace;
    `rico:isOrWasIncludedIn` domain = union(Record, RecordSet), range =
    RecordSet; `rico:isOrWasComponentOf` domain = Instantiation, range =
    Instantiation; `rico:IntellectualPropertyRightsRelation` and
    `rico:LegalStatus` are owl:Classes; the only class name containing
    "Right" is IntellectualPropertyRightsRelation.
18. NCAP sample revalidation after the interim-rights disclosure edit
    (pyshacl, advanced mode): with the adapter's interim rights statement
    attached, conforms; with all `naph:hasRightsStatement` triples removed,
    292 violations, every one on the rights constraint, none elsewhere.
19. Commerce profile negative controls re-run inside the public repository
    (`extensions/commerce/controls/run_controls.py`): licensable-no-offer
    fails, offer-no-route fails, offer-with-route passes; rebuild from
    `data/real-ncap-sample.ttl` reproduces records 292, per-item routes 292,
    fallbacks 0, triples 11,382, STAC items 292, and the built graph conforms
    to the commerce shapes.
20. STAC schema validation (2 September 2026, stac-validator over the
    repository's reports/stac/): catalog.json valid against the official
    STAC 1.0.0 catalog schema; all 292 items valid against the STAC 1.0.0
    item schema (292/292).
21. Interim Baseline rights statement changed from rightsstatements.org
    In Copyright to Copyright Not Evaluated (CNE/1.0, resolves 200), because
    the sample includes 55 records held for the United States National
    Archives whose copyright status only the institution can determine.
22. RiC-O 1.1: rico:Rule is an owl:Class and rico:Mandate rdfs:subClassOf
    rico:Rule (verified in RiC-O_1-1.rdf); Rule carries no URI-valued
    reuse-terms property.
23. CRMgeo citation verified via CrossRef (10.1007/s00799-016-0192-4):
    Hiebel, Doerr, Eide, IJDL 18(4):271-279, 2017. cidoc-crm.org resolves.
    WKT axis order verified longitude-first in data/real-ncap-sample.ttl
    (Hong Kong sample: 114.14, 22.36).
