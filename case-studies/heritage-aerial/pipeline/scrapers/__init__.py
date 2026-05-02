"""
NAPH scraper framework.

Each scraper produces NAPH-compliant Turtle from a specific external source.
All scrapers implement the same interface: input source descriptor → Turtle output.

Adapter pattern:
    - WikidataScraper          — public SPARQL, no auth (works out of the box)
    - NCAPAirPhotoFinderScraper — Angular SPA, requires Playwright/manual JSON capture (stub)
    - USGSEarthExplorerScraper — M2M API, requires registration (stub)
    - GenericCSVScraper         — file-based, see pipeline/ingest.py
"""

__all__ = [
    "wikidata",
    "ncap_airphotofinder",
    "usgs_earthexplorer",
]
