#!/usr/bin/env python3
"""
USGS Earth Explorer M2M API Adapter — STUB.

USGS hosts publicly-available declassified satellite imagery (CORONA, GAMBIT,
HEXAGON, KH-9) via the EarthExplorer service. The Machine-to-Machine (M2M)
API at https://m2m.cr.usgs.gov/ provides programmatic access.

The M2M API requires:
- A free USGS / ERS Registration System account
- Access permission for declassified-imagery datasets (granted on request)
- API authentication via login token

This file documents the adapter pattern. Implementation requires the user
to register and obtain credentials.

Documentation: https://m2m.cr.usgs.gov/api/docs/json/
Python client: https://pypi.org/project/usgs-m2m-api/

Usage (illustrative):
    export USGS_M2M_TOKEN=...
    python3 pipeline/scrapers/usgs_earthexplorer.py \
        --dataset declass_3 \
        --bbox -3.5,55.5,-3.0,56.0 \
        --start 1970-01-01 --end 1985-12-31

Mapping to NAPH:
- Each USGS scene → naph:AerialPhotograph (with naph:SatelliteAcquisition sortie)
- Geographic footprint → naph:GeographicFootprint with USGS-provided shapefile
- Acquisition date → naph:capturedOn
- Mission/platform/camera → naph:satelliteSystem, naph:cameraSystem, naph:missionNumber
- USGS entity ID → naph:downloadFromUSGS
- Rights → http://rightsstatements.org/vocab/NoC-US/1.0/
"""

import argparse
import sys


def main():
    parser = argparse.ArgumentParser(
        description="USGS Earth Explorer NAPH adapter (STUB — requires USGS credentials)."
    )
    parser.add_argument("--dataset", default="declass_3",
                        help="USGS dataset name (e.g. declass_3 for KH-9 partial)")
    parser.add_argument("--bbox", default=None,
                        help="Geographic bounding box: minlon,minlat,maxlon,maxlat")
    parser.add_argument("--start", default=None, help="Earliest acquisition date (YYYY-MM-DD)")
    parser.add_argument("--end", default=None, help="Latest acquisition date (YYYY-MM-DD)")
    parser.add_argument("--limit", type=int, default=100, help="Max scenes to fetch")
    args = parser.parse_args()

    print("# USGS Earth Explorer M2M adapter — STUB", file=sys.stderr)
    print("# Implementation requires:", file=sys.stderr)
    print("#   1. USGS ERS Registration: https://ers.cr.usgs.gov/register", file=sys.stderr)
    print("#   2. Access permission for declassified-imagery dataset", file=sys.stderr)
    print("#   3. M2M API authentication token (env: USGS_M2M_TOKEN)", file=sys.stderr)
    print("#", file=sys.stderr)
    print("# Once configured, the adapter will:", file=sys.stderr)
    print("#   - Authenticate via /api/login endpoint", file=sys.stderr)
    print("#   - Search the declassified-imagery datasets", file=sys.stderr)
    print("#   - Map each scene to naph:SatelliteAcquisition sortie + naph:AerialPhotograph", file=sys.stderr)
    print("#   - Output NAPH-compliant Turtle", file=sys.stderr)

    sys.exit(2)


if __name__ == "__main__":
    main()
