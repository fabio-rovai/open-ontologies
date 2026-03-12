#!/usr/bin/env python3
"""3-way comparison: Manual annotation vs Pure Claude vs RDF Pipeline.

Computes object recall, category recall, and structural metrics across
10 real photographs processed by parallel Claude agents.
"""
import json
import os
import glob

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DATASET_DIR = os.path.join(SCRIPT_DIR, "dataset")


def fuzzy_match(detected: str, ground_truth_set: set) -> bool:
    """Check if a detected label fuzzy-matches any ground truth label."""
    d = detected.lower().strip()
    for gt in ground_truth_set:
        g = gt.lower().strip()
        if d == g or d in g or g in d:
            return True
    return False


def compute_recall(detected: list, ground_truth: list) -> float:
    """What fraction of ground truth items were detected (fuzzy match)."""
    if not ground_truth:
        return 1.0
    gt_set = set(ground_truth)
    detected_lower = {d.lower().strip() for d in detected}
    hits = 0
    for gt in gt_set:
        g = gt.lower().strip()
        if any(g == d or g in d or d in g for d in detected_lower):
            hits += 1
    return hits / len(gt_set)


def compute_precision(detected: list, ground_truth: list) -> float:
    """What fraction of detected items match ground truth (fuzzy)."""
    if not detected:
        return 0.0
    gt_set = {g.lower().strip() for g in ground_truth}
    hits = 0
    for d in detected:
        dl = d.lower().strip()
        if any(dl == g or dl in g or g in dl for g in gt_set):
            hits += 1
    return hits / len(detected)


# Ground truth (manual annotation)
with open(os.path.join(DATASET_DIR, "ground_truth.json")) as f:
    ground_truth = json.load(f)

# Pure Claude results (from parallel agents)
pure_claude = {
    "img_1.jpg": {
        "objects": ["steering wheel", "dashboard", "windshield", "truck", "seat", "rust", "trees", "bushes", "foliage", "gauges"],
        "categories": ["vehicle", "nature", "decay"],
    },
    "img_2.jpg": {
        "objects": ["laptop", "notebook", "pen", "camera", "earbuds", "desk", "screen", "grid paper", "sketches", "wood table"],
        "categories": ["technology", "workspace", "stationery", "photography"],
    },
    "img_3.jpg": {
        "objects": ["frog", "duckweed", "water", "eyes"],
        "categories": ["animal", "nature", "water"],
    },
    "img_4.jpg": {
        "objects": ["trees", "forest", "hill", "field", "grass", "sky", "mist"],
        "categories": ["nature", "landscape", "forest"],
    },
    "img_5.jpg": {
        "objects": ["wooden deck", "planks", "nails", "buildings", "tower", "sky"],
        "categories": ["architecture", "nature", "urban"],
    },
    "img_6.jpg": {
        "objects": ["canoe", "paddle", "water", "people", "hat", "jacket", "fog", "reflection"],
        "categories": ["nature", "water", "recreation", "people"],
    },
    "img_7.jpg": {
        "objects": ["ocean", "beach", "coastline", "cliffs", "hills", "houses", "buildings", "trees", "vegetation", "sand", "waves", "roads", "golf course", "boat"],
        "categories": ["nature", "landscape", "coastal", "urban", "water"],
    },
    "img_8.jpg": {
        "objects": ["deer", "fawn", "spots", "ears", "trees", "branches", "leaves", "forest floor", "log", "sunlight"],
        "categories": ["animal", "nature", "wildlife", "forest"],
    },
    "img_9.jpg": {
        "objects": ["cat", "nose", "whiskers", "fur", "mouth", "nostrils"],
        "categories": ["animal", "pet"],
    },
    "img_10.jpg": {
        "objects": ["van", "windows", "tires", "wheels", "roof", "trees", "road", "leaves", "rust", "paint"],
        "categories": ["vehicle", "nature", "transportation"],
    },
}

# RDF Pipeline results (from parallel agents generating Turtle)
rdf_pipeline = {
    "img_1.jpg": {
        "objects": ["truck", "steering wheel", "dashboard", "windshield", "tree"],
        "categories": ["vehicle", "nature"],
        "triples": 88, "relationships": 6,
    },
    "img_2.jpg": {
        "objects": ["laptop", "notebook", "pen", "earbuds", "camera", "desk"],
        "categories": ["technology", "workspace", "photography"],
        "triples": 126, "relationships": 8,
    },
    "img_3.jpg": {
        "objects": ["frog", "frog eyes", "pond water", "duckweed", "frog head", "small insect", "frog nostrils"],
        "categories": ["amphibian", "animal body part", "water body", "aquatic plant", "insect"],
        "triples": 104, "relationships": 12,
    },
    "img_4.jpg": {
        "objects": ["overcast sky", "coniferous tree line", "grassy meadow", "distant hills", "haze", "bare trunks", "hill slope"],
        "categories": ["nature", "landscape", "vegetation", "terrain"],
        "triples": 97, "relationships": 8,
    },
    "img_5.jpg": {
        "objects": ["boardwalk surface", "wooden planks", "clear blue sky", "cylindrical tower", "background buildings", "horizon line"],
        "categories": ["architecture", "urban", "nature"],
        "triples": 82, "relationships": 6,
    },
    "img_6.jpg": {
        "objects": ["canoe", "stern paddler", "bow paddler", "paddle", "lake", "fog", "water reflection"],
        "categories": ["recreation", "water", "nature", "people"],
        "triples": 84, "relationships": 10,
    },
    "img_7.jpg": {
        "objects": ["ocean", "sandy beach", "sandstone cliffs", "golf course", "residential neighborhood", "hillside vegetation", "shallow water zone", "roads", "small vessel", "coastal canyon"],
        "categories": ["nature", "landscape", "coastal", "urban", "water", "geological formation"],
        "triples": 110, "relationships": 12,
    },
    "img_8.jpg": {
        "objects": ["fallow deer fawn", "forest floor", "deciduous forest trees", "golden backlight", "fallen tree branch", "airborne dust motes", "autumn foliage"],
        "categories": ["wildlife", "terrain", "vegetation", "lighting", "natural debris"],
        "triples": 94, "relationships": 8,
    },
    "img_9.jpg": {
        "objects": ["cat nose", "whisker pads", "philtrum", "fur", "nostrils", "nose bridge", "muzzle"],
        "categories": ["animal", "pet", "macro"],
        "triples": 106, "relationships": 8,
    },
    "img_10.jpg": {
        "objects": ["volkswagen type 2 van", "raised roof", "side windows", "front wheel", "rear wheel", "asphalt road", "foliage", "fallen leaves", "rust patches", "rear window"],
        "categories": ["vehicle", "nature", "transportation"],
        "triples": 98, "relationships": 10,
    },
}


def main():
    results = {}
    total_manual_obj = 0
    total_manual_cat = 0

    for img in sorted(ground_truth.keys()):
        gt = ground_truth[img]
        pc = pure_claude.get(img, {"objects": [], "categories": []})
        rp = rdf_pipeline.get(img, {"objects": [], "categories": [], "triples": 0, "relationships": 0})

        # Object recall
        pc_obj_recall = compute_recall(pc["objects"], gt["objects"])
        rp_obj_recall = compute_recall(rp["objects"], gt["objects"])

        # Category recall
        pc_cat_recall = compute_recall(pc["categories"], gt["categories"])
        rp_cat_recall = compute_recall(rp["categories"], gt["categories"])

        # Precision
        pc_obj_precision = compute_precision(pc["objects"], gt["objects"])
        rp_obj_precision = compute_precision(rp["objects"], gt["objects"])

        results[img] = {
            "gt_objects": len(gt["objects"]),
            "gt_categories": len(gt["categories"]),
            "pure_claude": {
                "objects_detected": len(pc["objects"]),
                "object_recall": round(pc_obj_recall, 2),
                "object_precision": round(pc_obj_precision, 2),
                "category_recall": round(pc_cat_recall, 2),
            },
            "rdf_pipeline": {
                "objects_detected": len(rp["objects"]),
                "object_recall": round(rp_obj_recall, 2),
                "object_precision": round(rp_obj_precision, 2),
                "category_recall": round(rp_cat_recall, 2),
                "triples": rp.get("triples", 0),
                "relationships": rp.get("relationships", 0),
            },
        }

    # Aggregate
    n = len(results)
    pc_avg_obj_recall = sum(r["pure_claude"]["object_recall"] for r in results.values()) / n
    pc_avg_cat_recall = sum(r["pure_claude"]["category_recall"] for r in results.values()) / n
    pc_avg_precision = sum(r["pure_claude"]["object_precision"] for r in results.values()) / n

    rp_avg_obj_recall = sum(r["rdf_pipeline"]["object_recall"] for r in results.values()) / n
    rp_avg_cat_recall = sum(r["rdf_pipeline"]["category_recall"] for r in results.values()) / n
    rp_avg_precision = sum(r["rdf_pipeline"]["object_precision"] for r in results.values()) / n
    rp_total_triples = sum(r["rdf_pipeline"]["triples"] for r in results.values())
    rp_total_rels = sum(r["rdf_pipeline"]["relationships"] for r in results.values())
    rp_avg_objects = sum(r["rdf_pipeline"]["objects_detected"] for r in results.values()) / n

    # Count TTL files
    ttl_files = glob.glob(os.path.join(DATASET_DIR, "*.ttl"))

    summary = {
        "dataset": "10 real photographs (picsum.photos)",
        "approach_comparison": {
            "manual_annotation": {
                "description": "Human expert labels objects and categories",
                "effort": "~2 min per image",
                "queryable": False,
                "relationships": False,
                "confidence_scores": False,
            },
            "pure_claude": {
                "description": "Claude vision returns JSON text labels",
                "avg_object_recall": round(pc_avg_obj_recall, 2),
                "avg_object_precision": round(pc_avg_precision, 2),
                "avg_category_recall": round(pc_avg_cat_recall, 2),
                "avg_objects_per_image": round(sum(r["pure_claude"]["objects_detected"] for r in results.values()) / n, 1),
                "queryable": False,
                "relationships": False,
                "confidence_scores": False,
            },
            "rdf_pipeline": {
                "description": "Claude vision → structured Turtle with ontology, validated with Open Ontologies",
                "avg_object_recall": round(rp_avg_obj_recall, 2),
                "avg_object_precision": round(rp_avg_precision, 2),
                "avg_category_recall": round(rp_avg_cat_recall, 2),
                "avg_objects_per_image": round(rp_avg_objects, 1),
                "total_triples": rp_total_triples,
                "total_relationships": rp_total_rels,
                "ttl_files_generated": len(ttl_files),
                "queryable": True,
                "relationships": True,
                "confidence_scores": True,
            },
        },
        "per_image": results,
    }

    out_path = os.path.join(DATASET_DIR, "benchmark_results.json")
    with open(out_path, "w") as f:
        json.dump(summary, f, indent=2)

    # Print comparison table
    print("=" * 90)
    print("3-WAY COMPARISON: Manual vs Pure Claude vs RDF Pipeline")
    print("=" * 90)
    print(f"{'Metric':<30} {'Manual':>12} {'Pure Claude':>14} {'RDF Pipeline':>14}")
    print("-" * 90)
    print(f"{'Object Recall':<30} {'100%':>12} {pc_avg_obj_recall*100:>13.0f}% {rp_avg_obj_recall*100:>13.0f}%")
    print(f"{'Object Precision':<30} {'100%':>12} {pc_avg_precision*100:>13.0f}% {rp_avg_precision*100:>13.0f}%")
    print(f"{'Category Recall':<30} {'100%':>12} {pc_avg_cat_recall*100:>13.0f}% {rp_avg_cat_recall*100:>13.0f}%")
    print(f"{'Avg Objects/Image':<30} {'—':>12} {sum(r['pure_claude']['objects_detected'] for r in results.values())/n:>13.1f} {rp_avg_objects:>13.1f}")
    print(f"{'Total RDF Triples':<30} {'0':>12} {'0':>14} {rp_total_triples:>14}")
    print(f"{'Spatial Relationships':<30} {'0':>12} {'0':>14} {rp_total_rels:>14}")
    print(f"{'SPARQL Queryable':<30} {'No':>12} {'No':>14} {'Yes':>14}")
    print(f"{'Confidence Scores':<30} {'No':>12} {'No':>14} {'Yes':>14}")
    print(f"{'Effort per Image':<30} {'~2 min':>12} {'~8 sec':>14} {'~8 sec':>14}")
    print(f"{'Scales to 1000 images':<30} {'No':>12} {'Yes':>14} {'Yes':>14}")
    print("=" * 90)
    print(f"\nResults saved to {out_path}")


if __name__ == "__main__":
    main()
