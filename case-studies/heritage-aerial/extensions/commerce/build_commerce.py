#!/usr/bin/env python3
"""
Lift the real NCAP Baseline sample into the NAPH Commerce Profile.

Takes the 292 real frames already harvested from the public Air Photo Finder
API and lifted to NAPH Baseline, and adds the one thing the Baseline rights
module cannot express: that the record is rights-reserved AND licensable AND
here is the per-item route to acquire that licence.

The route is not invented. Every harvested record already carries its own
catalogue URL as prov:hadPrimarySource, e.g.

    <https://airphotofinder.ncap.org/image/797810>

which is the page from which a licence for that exact frame can be ordered.
The commerce profile does nothing more than make that fact machine-readable
and attach ODRL semantics to it, so that an aggregator, a search engine or a
research pipeline can act on it instead of reading "In Copyright" and dropping
the record.

Outputs:
    build/ncap-commerce.ttl   Baseline sample + commerce profile
    build/stac/catalog.json   STAC catalogue with licence and order links
    build/report.json         Counts for verification

Usage:
    python3 pipeline/build_commerce.py \
        --source ~/projects/open-ontologies/case-studies/heritage-aerial/data/real-ncap-sample.ttl
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import sys

from rdflib import Graph, Namespace, Literal, URIRef, RDF, RDFS
from rdflib.namespace import XSD, DCTERMS, SKOS

NAPH = Namespace("https://w3id.org/naph/ontology#")
NAPHC = Namespace("https://w3id.org/naph/commerce#")
ODRL = Namespace("http://www.w3.org/ns/odrl/2/")
PROV = Namespace("http://www.w3.org/ns/prov#")
GEO = Namespace("http://www.opengis.net/ont/geosparql#")
EX = Namespace("https://w3id.org/naph/example/ncap-live/")

# The assigner: the body entitled to issue the licence.
NCAP_PARTY = EX["party-HES"]

# Collection-level fallback, used where a per-item catalogue URL is absent.
ENQUIRY = "https://www.ncap.org/support/contact-us"

# Metadata licence proposed for the discovery surface. Deliberately left as a
# proposal rather than an assertion: only NCAP can set this.
METADATA_LICENCE = URIRef("https://creativecommons.org/licenses/by/4.0/")


def build(source: pathlib.Path) -> tuple[Graph, dict]:
    g = Graph()
    g.parse(source, format="turtle")

    for prefix, ns in [("naphc", NAPHC), ("odrl", ODRL), ("skos", SKOS)]:
        g.bind(prefix, ns)

    # The assigner party.
    g.add((NCAP_PARTY, RDF.type, ODRL.Party))
    g.add((NCAP_PARTY, RDFS.label,
           Literal("Historic Environment Scotland (National Collection of "
                   "Aerial Photography)")))
    g.add((NCAP_PARTY, RDFS.seeAlso, URIRef("https://www.ncap.org/")))

    photos = list(g.subjects(RDF.type, NAPH.AerialPhotograph))
    with_route = 0
    fallback = 0

    for photo in photos:
        # Rights are reserved, and the item is licensable. These are different
        # statements and the profile keeps them separate.
        g.add((photo, NAPHC.availability, NAPHC.Licensable))

        local = str(photo).rsplit("/", 1)[-1]
        offer = EX[f"offer-{local}"]
        g.add((photo, NAPHC.hasOffer, offer))
        g.add((offer, RDF.type, NAPHC.ReproductionOffer))
        g.add((offer, ODRL.target, photo))
        g.add((offer, ODRL.assigner, NCAP_PARTY))

        # What is actually on sale: the right to reproduce a supplied scan.
        permission = EX[f"perm-{local}"]
        g.add((offer, ODRL.permission, permission))
        g.add((permission, ODRL.action, ODRL.reproduce))
        g.add((permission, ODRL.target, photo))
        g.add((permission, ODRL.assigner, NCAP_PARTY))

        # The duty that makes it a commercial offer rather than a grant.
        duty = EX[f"duty-{local}"]
        g.add((permission, ODRL.duty, duty))
        g.add((duty, ODRL.action, ODRL.compensate))
        g.add((duty, RDFS.comment,
               Literal("Licence fee payable to the holding institution. "
                       "Amount is set per order at the catalogue endpoint; no "
                       "price is asserted here.")))

        # The route. Reuse the catalogue URL the harvest already recorded.
        source_page = g.value(photo, PROV.hadPrimarySource)
        if source_page is not None:
            g.add((offer, NAPHC.orderEndpoint,
                   Literal(str(source_page), datatype=XSD.anyURI)))
            with_route += 1
        else:
            g.add((offer, NAPHC.enquiryEndpoint,
                   Literal(ENQUIRY, datatype=XSD.anyURI)))
            fallback += 1

        g.add((offer, NAPHC.scanOnDemand, Literal(True)))

        # Discovery surface: what a third party may index for free.
        surface = EX[f"surface-{local}"]
        g.add((photo, NAPHC.hasDiscoverySurface, surface))
        g.add((surface, RDF.type, NAPHC.DiscoverySurface))
        g.add((surface, NAPHC.metadataLicence, METADATA_LICENCE))
        g.add((surface, NAPHC.previewMaxPixels, Literal(1024)))

    report = {
        "records": len(photos),
        "offers_with_per_item_order_route": with_route,
        "offers_with_collection_fallback": fallback,
        "triples": len(g),
    }
    return g, report


def build_stac(g: Graph) -> dict:
    """Emit a STAC catalogue in which the licence is a first-class field and
    the order route is a typed link. This is the surface that puts the
    collection inside the geospatial toolchain without giving away a pixel."""
    items = []
    for photo in g.subjects(RDF.type, NAPH.AerialPhotograph):
        fp = g.value(photo, NAPH.coversArea)
        wkt = g.value(fp, NAPH.asWKT) if fp else None
        date = g.value(photo, NAPH.capturedOn)
        offer = g.value(photo, NAPHC.hasOffer)
        order = g.value(offer, NAPHC.orderEndpoint) if offer else None
        enquiry = g.value(offer, NAPHC.enquiryEndpoint) if offer else None
        label = g.value(photo, RDFS.label)

        links = [{"rel": "license",
                  "href": "http://rightsstatements.org/vocab/InC/1.0/",
                  "title": "In Copyright"}]
        route = order or enquiry
        if route is not None:
            links.append({"rel": "order", "href": str(route),
                          "title": "Licence this frame"})

        items.append({
            "type": "Feature",
            "stac_version": "1.0.0",
            "id": str(photo).rsplit("/", 1)[-1],
            "properties": {
                "datetime": f"{date}T00:00:00Z" if date else None,
                "title": str(label) if label else None,
                "license": "proprietary",
                "naph:availability": "licensable",
                "naph:wkt": str(wkt) if wkt else None,
            },
            "links": links,
            "assets": {},
        })

    return {
        "type": "Catalog",
        "stac_version": "1.0.0",
        "id": "ncap-commerce-demo",
        "description": ("NCAP Baseline sample with machine-readable licensing "
                        "offers. Metadata and footprints are open for "
                        "discovery; the imagery is licensed."),
        "license": "proprietary",
        "links": [{"rel": "self", "href": "./catalog.json"}],
        "item_count": len(items),
        "items": items,
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--source", required=True,
                    help="Path to real-ncap-sample.ttl (NAPH Baseline)")
    ap.add_argument("--outdir", default="build")
    args = ap.parse_args()

    src = pathlib.Path(os.path.expanduser(args.source))
    if not src.is_file():
        print(f"source not found: {src}", file=sys.stderr)
        return 1

    out = pathlib.Path(args.outdir)
    (out / "stac").mkdir(parents=True, exist_ok=True)

    g, report = build(src)
    g.serialize(destination=out / "ncap-commerce.ttl", format="turtle")

    catalog = build_stac(g)
    (out / "stac" / "catalog.json").write_text(json.dumps(catalog, indent=2))
    report["stac_items"] = catalog["item_count"]

    (out / "report.json").write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
