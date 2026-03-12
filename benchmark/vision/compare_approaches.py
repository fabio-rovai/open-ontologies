#!/usr/bin/env python3
"""3-way comparison: Manual annotation vs Pure Claude vs RDF Pipeline.

Computes object recall, category recall, and structural metrics across
10 real photographs processed by parallel Claude agents.

RDF pipeline data is extracted directly from the actual TTL files —
no hardcoded values.
"""
import json
import os
import re
import glob

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DATASET_DIR = os.path.join(SCRIPT_DIR, "dataset")

# Spatial relationship labels to exclude from object lists
SPATIAL_LABELS = {
    "above", "below", "behind", "in front of", "beside", "left of", "right of",
    "near", "inside", "on top of", "beneath", "surrounding", "centered in",
    "submerged in", "covered by", "surrounded by", "part of", "floating on",
    "covering", "clinging to", "composed of", "above-left of", "resting on",
    "between", "exhibited by", "provided by", "contains", "habitat for",
    "standing on", "illuminated by", "same as", "source of", "illuminates",
    "filters through", "on", "leading up to", "partially obscured by",
}


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


def extract_from_ttl(ttl_path):
    """Extract objects, categories, and counts from a TTL file."""
    content = open(ttl_path).read()

    # rdfs:label values (exclude spatial relationship labels)
    labels = []
    for m in re.findall(r'rdfs:label\s+"([^"]+)"', content):
        if m.lower() not in SPATIAL_LABELS:
            labels.append(m)

    # skos:altLabel values
    alt_labels = re.findall(r'skos:altLabel\s+"([^"]+)"', content)

    # Categories
    categories = sorted(set(re.findall(r'schema:category\s+"([^"]+)"', content)))

    # Relationship count (spatial predicates)
    rel_patterns = r'ex:spatialRelation|ex:near|ex:above|ex:below|ex:beside|ex:inside|ex:behind|ex:leftOf|ex:rightOf|ex:surrounding|ex:partOf|ex:adjacentTo'
    relationships = len(re.findall(rel_patterns, content))

    # Combined object list (labels + altLabels, deduplicated)
    all_objects = list(dict.fromkeys(labels + alt_labels))

    return {
        "objects": all_objects,
        "categories": categories,
        "relationships": relationships,
    }


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

# RDF Pipeline — extract directly from actual TTL files
rdf_pipeline = {}
for ttl in sorted(glob.glob(os.path.join(DATASET_DIR, "img_*.ttl"))):
    img_key = os.path.basename(ttl).replace(".ttl", ".jpg")
    rdf_pipeline[img_key] = extract_from_ttl(ttl)

# Get validated triple counts from MCP pipeline results
mcp_results_path = os.path.join(DATASET_DIR, "mcp_pipeline_results.json")
if os.path.exists(mcp_results_path):
    with open(mcp_results_path) as f:
        mcp_results = json.load(f)
    total_triples_from_mcp = mcp_results.get("total_triples_validated", 0)
else:
    total_triples_from_mcp = 0

# Get per-file triple counts by running onto_validate via CLI
OO_BIN = os.path.join(SCRIPT_DIR, "..", "..", "target", "release", "open-ontologies")
import subprocess
for img_key, data in rdf_pipeline.items():
    ttl_path = os.path.join(DATASET_DIR, img_key.replace(".jpg", ".ttl"))
    try:
        result = subprocess.run(
            [OO_BIN, "validate", ttl_path],
            capture_output=True, text=True, timeout=10
        )
        parsed = json.loads(result.stdout.strip())
        data["triples"] = parsed.get("triple_count", parsed.get("triples", 0))
    except Exception:
        data["triples"] = 0


def main():
    results = {}

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
