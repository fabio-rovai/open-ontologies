#!/usr/bin/env python3
"""
Download OAEI benchmark data (Anatomy + Conference tracks).

Sources:
- Anatomy: http://oaei.ontologymatching.org/2023/anatomy/
- Conference: http://oaei.ontologymatching.org/2023/conference/
"""
import os
import urllib.request
import zipfile
import shutil

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DATA_DIR = os.path.join(SCRIPT_DIR, "data")

# OAEI Anatomy track — download as zip archive (stable URL from 2019, same dataset used through 2024)
ANATOMY_ZIP_URL = "http://oaei.ontologymatching.org/2019/anatomy/anatomy-dataset.zip"

# OAEI Conference track — download as zip archive
CONFERENCE_ZIP_URL = "http://oaei.ontologymatching.org/2019/conference/conference.zip"


def download_and_extract_zip(url: str, dest_dir: str) -> bool:
    """Download a zip file and extract it to dest_dir."""
    zip_path = os.path.join(dest_dir, "download.zip")
    try:
        print(f"  [download] {url}")
        urllib.request.urlretrieve(url, zip_path)
        print(f"  [extract] -> {dest_dir}")
        with zipfile.ZipFile(zip_path, "r") as zf:
            zf.extractall(dest_dir)
        os.remove(zip_path)
        return True
    except Exception as e:
        print(f"  [FAIL] {url}: {e}")
        return False


def download_anatomy():
    """Download OAEI Anatomy track (mouse + human + reference alignment)."""
    print("\n=== Anatomy Track ===")
    anatomy_dir = os.path.join(DATA_DIR, "anatomy")
    os.makedirs(anatomy_dir, exist_ok=True)

    # Check if already downloaded
    if os.path.exists(os.path.join(anatomy_dir, "mouse.owl")):
        print("  [skip] Anatomy data already exists")
        return

    download_and_extract_zip(ANATOMY_ZIP_URL, anatomy_dir)

    # List what we got
    for f in sorted(os.listdir(anatomy_dir)):
        size = os.path.getsize(os.path.join(anatomy_dir, f))
        print(f"  {f} ({size:,} bytes)")


def download_conference():
    """Download OAEI Conference track ontologies + reference alignments."""
    print("\n=== Conference Track ===")
    conf_dir = os.path.join(DATA_DIR, "conference")
    os.makedirs(conf_dir, exist_ok=True)

    # Check if already downloaded
    if any(f.endswith(".owl") for f in os.listdir(conf_dir) if os.path.isfile(os.path.join(conf_dir, f))):
        print("  [skip] Conference data already exists")
        return

    download_and_extract_zip(CONFERENCE_ZIP_URL, conf_dir)

    # Count what we got
    owl_count = sum(1 for root, _, files in os.walk(conf_dir) for f in files if f.endswith(".owl"))
    rdf_count = sum(1 for root, _, files in os.walk(conf_dir) for f in files if f.endswith(".rdf"))
    print(f"  {owl_count} ontologies, {rdf_count} reference alignments")


if __name__ == "__main__":
    print("Downloading OAEI benchmark data...")
    os.makedirs(DATA_DIR, exist_ok=True)
    download_anatomy()
    download_conference()
    print("\nDone. Data in:", DATA_DIR)
