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
