#!/usr/bin/env python3
"""Resize images to optimal size for Claude vision. Minimal tokens, max understanding."""
import os
import sys
import struct
import zlib
from concurrent.futures import ThreadPoolExecutor

MAX_DIM = 256  # Claude understands fine at this size, 4x fewer tokens than 512


def read_png_dimensions(path):
    """Read PNG width/height from header without PIL."""
    with open(path, "rb") as f:
        sig = f.read(8)
        if sig[:4] != b"\x89PNG":
            return None, None
        f.read(4)  # chunk length
        f.read(4)  # IHDR
        w = struct.unpack(">I", f.read(4))[0]
        h = struct.unpack(">I", f.read(4))[0]
        return w, h


def resize_with_sips(src, dst, max_dim):
    """Use macOS sips (built-in, no dependencies) to resize."""
    w, h = read_png_dimensions(src)
    if w and h and max(w, h) <= max_dim:
        os.link(src, dst) if not os.path.exists(dst) else None
        return
    # sips resizes keeping aspect ratio with --resampleHeightWidthMax
    os.system(f'sips --resampleHeightWidthMax {max_dim} "{src}" --out "{dst}" >/dev/null 2>&1')


def process_dir(input_dir, output_dir, max_dim=MAX_DIM):
    os.makedirs(output_dir, exist_ok=True)
    images = [
        f for f in os.listdir(input_dir)
        if f.lower().endswith((".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp"))
    ]
    if not images:
        print(f"No images found in {input_dir}")
        return []

    def resize_one(name):
        src = os.path.join(input_dir, name)
        dst = os.path.join(output_dir, os.path.splitext(name)[0] + ".png")
        resize_with_sips(src, dst, max_dim)
        return dst

    with ThreadPoolExecutor(max_workers=os.cpu_count()) as pool:
        results = list(pool.map(resize_one, images))

    print(f"Resized {len(results)} images to {output_dir} (max {max_dim}px)")
    return results


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: resize.py <input_dir> [output_dir] [max_dim]")
        sys.exit(1)
    input_dir = sys.argv[1]
    output_dir = sys.argv[2] if len(sys.argv) > 2 else os.path.join(input_dir, "resized")
    max_dim = int(sys.argv[3]) if len(sys.argv) > 3 else MAX_DIM
    process_dir(input_dir, output_dir, max_dim)
