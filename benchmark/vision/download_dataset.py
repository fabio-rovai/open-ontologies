#!/usr/bin/env python3
"""Download a small annotated image dataset for benchmarking.
Uses Wikimedia Commons thumbnails (public domain) with known subjects."""
import os
import json
import urllib.request

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
OUT_DIR = os.path.join(SCRIPT_DIR, "dataset")

# 10 diverse images from Wikimedia Commons (public domain / CC0)
# Each has manual ground truth labels
IMAGES = [
    {
        "file": "airplane.jpg",
        "url": "https://upload.wikimedia.org/wikipedia/commons/thumb/4/4e/Cessna_172_Skyhawk_%28D-ECHO%29_02.jpg/320px-Cessna_172_Skyhawk_%28D-ECHO%29_02.jpg",
        "labels": ["airplane", "aircraft", "vehicle", "sky"],
        "source": "Wikimedia Commons",
    },
    {
        "file": "cat.jpg",
        "url": "https://upload.wikimedia.org/wikipedia/commons/thumb/4/4d/Cat_November_2010-1a.jpg/220px-Cat_November_2010-1a.jpg",
        "labels": ["cat", "animal", "pet", "feline"],
        "source": "Wikimedia Commons",
    },
    {
        "file": "car.jpg",
        "url": "https://upload.wikimedia.org/wikipedia/commons/thumb/1/1b/2019_Honda_Civic_sedan_%28facelift%2C_red%29%2C_front_9.29.19.jpg/320px-2019_Honda_Civic_sedan_%28facelift%2C_red%29%2C_front_9.29.19.jpg",
        "labels": ["car", "automobile", "vehicle", "red"],
        "source": "Wikimedia Commons",
    },
    {
        "file": "flower.jpg",
        "url": "https://upload.wikimedia.org/wikipedia/commons/thumb/4/41/Sunflower_from_Silesia2.jpg/220px-Sunflower_from_Silesia2.jpg",
        "labels": ["flower", "sunflower", "plant", "yellow"],
        "source": "Wikimedia Commons",
    },
    {
        "file": "bicycle.jpg",
        "url": "https://upload.wikimedia.org/wikipedia/commons/thumb/a/a7/Camponotus_flavomarginatus_ant.jpg/320px-Camponotus_flavomarginatus_ant.jpg",
        "labels": ["ant", "insect", "animal", "macro"],
        "source": "Wikimedia Commons",
    },
    {
        "file": "dog.jpg",
        "url": "https://upload.wikimedia.org/wikipedia/commons/thumb/2/26/YellowLabradorLooking_new.jpg/220px-YellowLabradorLooking_new.jpg",
        "labels": ["dog", "animal", "pet", "labrador"],
        "source": "Wikimedia Commons",
    },
    {
        "file": "bridge.jpg",
        "url": "https://upload.wikimedia.org/wikipedia/commons/thumb/0/0c/GoldenGateBridge-001.jpg/320px-GoldenGateBridge-001.jpg",
        "labels": ["bridge", "structure", "golden gate", "architecture"],
        "source": "Wikimedia Commons",
    },
    {
        "file": "guitar.jpg",
        "url": "https://upload.wikimedia.org/wikipedia/commons/thumb/4/45/GuitareClassique5.png/120px-GuitareClassique5.png",
        "labels": ["guitar", "instrument", "music", "strings"],
        "source": "Wikimedia Commons",
    },
    {
        "file": "mountain.jpg",
        "url": "https://upload.wikimedia.org/wikipedia/commons/thumb/e/e7/Everest_North_Face_toward_Base_Camp_Tibet_Luca_Galuzzi_2006.jpg/320px-Everest_North_Face_toward_Base_Camp_Tibet_Luca_Galuzzi_2006.jpg",
        "labels": ["mountain", "landscape", "snow", "nature", "everest"],
        "source": "Wikimedia Commons",
    },
    {
        "file": "clock.jpg",
        "url": "https://upload.wikimedia.org/wikipedia/commons/thumb/a/a0/Clock_Tower_-_Palace_of_Westminster%2C_London_-_September_2006.jpg/180px-Clock_Tower_-_Palace_of_Westminster%2C_London_-_September_2006.jpg",
        "labels": ["clock", "tower", "big ben", "architecture", "london"],
        "source": "Wikimedia Commons",
    },
]


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    ground_truth = {}

    for img in IMAGES:
        path = os.path.join(OUT_DIR, img["file"])
        if not os.path.exists(path):
            print(f"Downloading {img['file']}...")
            try:
                req = urllib.request.Request(img["url"], headers={"User-Agent": "Mozilla/5.0"})
                with urllib.request.urlopen(req, timeout=15) as resp:
                    with open(path, "wb") as f:
                        f.write(resp.read())
            except Exception as e:
                print(f"  SKIP: {e}")
                continue
        ground_truth[img["file"]] = img["labels"]
        size = os.path.getsize(path)
        print(f"  {img['file']}: {size:,} bytes, labels: {img['labels']}")

    gt_path = os.path.join(OUT_DIR, "ground_truth.json")
    with open(gt_path, "w") as f:
        json.dump(ground_truth, f, indent=2)
    print(f"\nGround truth saved to {gt_path}")
    print(f"Total: {len(ground_truth)} images ready")


if __name__ == "__main__":
    main()
