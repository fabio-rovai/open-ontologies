#!/usr/bin/env python3
"""
NCAP Air Photo Finder Adapter — STUB.

Air Photo Finder (https://airphotofinder.ncap.org/) is an Angular-based SPA.
The data is loaded asynchronously by JavaScript from internal API endpoints.

Without browser automation (Playwright, Selenium) or institutional API access,
direct scraping is not feasible. This file documents the adapter pattern so an
institution with appropriate access can complete the integration.

Three feasible pathways for an institution:

1. **Institutional API access** — NCAP / HES may expose internal APIs to
   institutional partners under specific agreements. Contact:
   ncap@hes.scot

2. **Browser-automation scraping** — Playwright or Puppeteer can drive the
   SPA, capture network requests, and extract metadata. This works but:
   - Requires bot-friendly engagement with the institution
   - Subject to AI-bot blocking policies (NCAP main domain blocks GPTBot
     and other AI crawlers — see /robots.txt)
   - Rate limiting essential

3. **CSV export from internal cataloguing systems** — institutions can use
   pipeline/ingest.py directly on their own catalogue exports without
   needing to scrape their public site. This is the recommended pathway.

This stub is non-functional but provides the structural template:
- Configuration: target collection, sample size, output path
- Output: NAPH-compliant Turtle to stdout

Usage (illustrative — does not run):
    python3 pipeline/scrapers/ncap_airphotofinder.py --collection RAF --limit 100
"""

import argparse
import sys


def main():
    parser = argparse.ArgumentParser(
        description="NCAP Air Photo Finder NAPH adapter (STUB — see file header for guidance)."
    )
    parser.add_argument("--collection", default="RAF",
                        help="NCAP collection code to scrape")
    parser.add_argument("--limit", type=int, default=100,
                        help="Maximum number of records to fetch")
    parser.add_argument("--api-key", default=None,
                        help="Institutional API key (required if NCAP exposes one)")
    args = parser.parse_args()

    print("# NCAP Air Photo Finder adapter — STUB", file=sys.stderr)
    print("# Air Photo Finder is an Angular SPA without a documented public API.", file=sys.stderr)
    print("# To complete this integration, an institution needs one of:", file=sys.stderr)
    print("#   1. Institutional API access via partnership with NCAP / HES", file=sys.stderr)
    print("#   2. Browser-automation scraping (Playwright) with institutional permission", file=sys.stderr)
    print("#   3. Direct CSV export from internal cataloguing systems (preferred)", file=sys.stderr)
    print("#", file=sys.stderr)
    print("# For (3), use pipeline/ingest.py directly on your CSV export.", file=sys.stderr)
    print("# Contact: ncap@hes.scot for institutional partnerships.", file=sys.stderr)

    sys.exit(2)


if __name__ == "__main__":
    main()
