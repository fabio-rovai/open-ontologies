#!/usr/bin/env python3
"""
Field-of-View Footprint Derivation

Derives accurate WGS84 polygon footprints for vertical aerial photographs
from altitude, focal length, and image format.

Implements the geometric derivation described in the Aerial Photography Profile §P.4.

Usage:
    python3 footprint-from-flight.py --lat 55.953 --lon -3.188 \\
        --altitude-m 9144 --focal-length-mm 914 --image-edge-mm 230

Output: WGS84 POLYGON WKT to stdout.
"""

import argparse
import math
import sys


def metres_per_degree_lat() -> float:
    """Approximate metres per degree of latitude (constant globally)."""
    return 111_111.0


def metres_per_degree_lon(latitude_deg: float) -> float:
    """Metres per degree of longitude at a given latitude."""
    return 111_111.0 * math.cos(math.radians(latitude_deg))


def derive_vertical_footprint(
    centre_lat: float,
    centre_lon: float,
    altitude_m: float,
    focal_length_mm: float,
    image_edge_mm: float = 230.0,
    heading_deg: float = 0.0,
) -> list[tuple[float, float]]:
    """
    Derive a vertical-photography ground footprint as 4 corners + closing point.

    Args:
        centre_lat: latitude of the photograph's principal point (decimal degrees)
        centre_lon: longitude of the photograph's principal point (decimal degrees)
        altitude_m: flight altitude above ground level (metres)
        focal_length_mm: camera focal length (mm)
        image_edge_mm: physical image edge dimension (mm). Default 230mm for standard
            9-inch aerial format. Use 224mm for KH-9 panoramic, etc.
        heading_deg: aircraft heading at time of capture (degrees, 0=North).
            Used to rotate the footprint to match flight direction.

    Returns:
        List of (longitude, latitude) tuples forming a closed polygon.
    """
    if altitude_m <= 0:
        raise ValueError(f"altitude must be positive, got {altitude_m}")
    if focal_length_mm <= 0:
        raise ValueError(f"focal length must be positive, got {focal_length_mm}")
    if not -90 <= centre_lat <= 90:
        raise ValueError(f"latitude out of range: {centre_lat}")
    if not -180 <= centre_lon <= 180:
        raise ValueError(f"longitude out of range: {centre_lon}")

    # Ground distance per image edge (metres)
    ground_edge_m = (image_edge_mm * altitude_m) / focal_length_mm

    half_edge_m = ground_edge_m / 2.0

    # Convert metric offsets to lat/lon offsets
    deg_per_metre_lat = 1.0 / metres_per_degree_lat()
    deg_per_metre_lon = 1.0 / metres_per_degree_lon(centre_lat)

    half_lat_deg = half_edge_m * deg_per_metre_lat
    half_lon_deg = half_edge_m * deg_per_metre_lon

    # Define four corners (relative to centre, in local metres)
    # Order: NW, NE, SE, SW, NW (closing)
    corners_local = [
        (-half_edge_m, +half_edge_m),  # NW
        (+half_edge_m, +half_edge_m),  # NE
        (+half_edge_m, -half_edge_m),  # SE
        (-half_edge_m, -half_edge_m),  # SW
        (-half_edge_m, +half_edge_m),  # NW (close)
    ]

    # Rotate by heading (heading rotates the rectangle in flight direction)
    heading_rad = math.radians(heading_deg)
    cos_h = math.cos(heading_rad)
    sin_h = math.sin(heading_rad)

    polygon = []
    for east_m, north_m in corners_local:
        # Rotate
        east_rot = east_m * cos_h - north_m * sin_h
        north_rot = east_m * sin_h + north_m * cos_h
        # Convert to lat/lon
        lat = centre_lat + (north_rot * deg_per_metre_lat)
        lon = centre_lon + (east_rot * deg_per_metre_lon)
        polygon.append((lon, lat))

    return polygon


def derive_ground_sample_distance(
    altitude_m: float,
    focal_length_mm: float,
    pixel_pitch_mm: float,
) -> float:
    """
    Derive the ground sample distance (metres per pixel) for a digitised aerial photograph.

    Args:
        altitude_m: flight altitude AGL (m)
        focal_length_mm: camera focal length (mm)
        pixel_pitch_mm: pixel pitch — for film: 25.4 / dpi. For digital sensor: physical pitch.

    Returns:
        GSD in metres per pixel.

    Example for F.52 at 30,000 ft (9144 m), scanned at 1200 DPI:
        pixel_pitch_mm = 25.4 / 1200 = 0.02117
        focal_length_mm = 914.4
        altitude_m = 9144
        GSD = 0.02117 * 9144 / 914.4 = 0.21 m/pixel
    """
    return (pixel_pitch_mm * altitude_m) / focal_length_mm


def to_wkt(polygon: list[tuple[float, float]]) -> str:
    """Convert a list of (lon, lat) tuples to WKT POLYGON syntax."""
    coords = ", ".join(f"{lon:.6f} {lat:.6f}" for lon, lat in polygon)
    return f"POLYGON(({coords}))"


def main():
    parser = argparse.ArgumentParser(
        description="Derive WGS84 polygon footprint for a vertical aerial photograph."
    )
    parser.add_argument("--lat", type=float, required=True,
                        help="Centre latitude (decimal degrees)")
    parser.add_argument("--lon", type=float, required=True,
                        help="Centre longitude (decimal degrees)")
    parser.add_argument("--altitude-m", type=float, required=True,
                        help="Flight altitude AGL in metres")
    parser.add_argument("--focal-length-mm", type=float, required=True,
                        help="Camera focal length in mm")
    parser.add_argument("--image-edge-mm", type=float, default=230.0,
                        help="Image format edge in mm (default 230 for 9-inch aerial)")
    parser.add_argument("--heading-deg", type=float, default=0.0,
                        help="Aircraft heading at capture (degrees, 0=North; default 0)")
    parser.add_argument("--gsd", action="store_true",
                        help="Also compute and print Ground Sample Distance (requires --pixel-pitch-mm)")
    parser.add_argument("--pixel-pitch-mm", type=float, default=None,
                        help="Pixel pitch in mm for GSD calculation")
    args = parser.parse_args()

    polygon = derive_vertical_footprint(
        centre_lat=args.lat,
        centre_lon=args.lon,
        altitude_m=args.altitude_m,
        focal_length_mm=args.focal_length_mm,
        image_edge_mm=args.image_edge_mm,
        heading_deg=args.heading_deg,
    )

    print(to_wkt(polygon))

    if args.gsd:
        if args.pixel_pitch_mm is None:
            print("# --gsd requires --pixel-pitch-mm", file=sys.stderr)
            sys.exit(1)
        gsd = derive_ground_sample_distance(
            altitude_m=args.altitude_m,
            focal_length_mm=args.focal_length_mm,
            pixel_pitch_mm=args.pixel_pitch_mm,
        )
        print(f"# Ground Sample Distance: {gsd:.4f} m/pixel", file=sys.stderr)


if __name__ == "__main__":
    main()
