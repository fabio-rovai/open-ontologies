#!/usr/bin/env python3
"""Full pipeline benchmark: validate → load → query across all 10 images.

Demonstrates the complete Open Ontologies workflow:
1. onto_validate — syntax check each TTL (via CLI)
2. Load all into unified graph (rdflib)
3. SPARQL queries across the combined knowledge graph
4. Compare against ground truth
"""
import json
import os
import subprocess
import glob

try:
    from rdflib import Graph, Namespace
except ImportError:
    print("pip install rdflib")
    raise

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DATASET_DIR = os.path.join(SCRIPT_DIR, "dataset")
OO_BIN = os.path.join(SCRIPT_DIR, "..", "..", "target", "release", "open-ontologies")

EX = Namespace("http://example.org/image/")
SCHEMA = Namespace("http://schema.org/")
SKOS = Namespace("http://www.w3.org/2004/02/skos/core#")
RDFS = Namespace("http://www.w3.org/2000/01/rdf-schema#")


def validate_with_oo(ttl_path):
    """Run onto_validate via CLI binary."""
    result = subprocess.run(
        [OO_BIN, "validate", ttl_path],
        capture_output=True, text=True
    )
    data = json.loads(result.stdout.strip())
    return data.get("ok", False), data.get("triples", 0)


def main():
    ttl_files = sorted(glob.glob(os.path.join(DATASET_DIR, "img_*.ttl")))
    gt = json.load(open(os.path.join(DATASET_DIR, "ground_truth.json")))

    print("=" * 80)
    print("FULL PIPELINE: onto_validate → load → SPARQL query")
    print("=" * 80)

    # Step 1: Validate each TTL with Open Ontologies CLI
    print("\n--- Step 1: onto_validate (Open Ontologies CLI) ---")
    total_triples = 0
    validated = []
    for ttl in ttl_files:
        ok, triples = validate_with_oo(ttl)
        status = "VALID" if ok else "FAILED"
        print(f"  {os.path.basename(ttl)}: {status} — {triples} triples")
        total_triples += triples
        if ok:
            validated.append(ttl)
    print(f"  Total: {total_triples} triples, {len(validated)}/10 valid")

    # Step 2: Load all into unified graph
    print("\n--- Step 2: Load into unified RDF graph ---")
    g = Graph()
    for ttl in validated:
        g.parse(ttl, format="turtle")
    print(f"  Loaded {len(g)} triples from {len(validated)} files")

    # Step 3: SPARQL queries
    print("\n--- Step 3: SPARQL queries across combined graph ---")

    # Query 1: All images with descriptions
    q1 = g.query("""
        PREFIX schema: <http://schema.org/>
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        SELECT ?img ?desc WHERE {
            ?img a schema:ImageObject .
            ?img schema:description ?desc .
        }
    """)
    print(f"\n  Query: Images with descriptions → {len(q1)} results")
    for row in q1:
        img_name = str(row.img).split("/")[-1]
        desc = str(row.desc)[:80]
        print(f"    {img_name}: {desc}...")

    # Query 2: All objects with confidence > 0.9
    q2 = g.query("""
        PREFIX ex: <http://example.org/image/>
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        SELECT ?label ?conf WHERE {
            ?obj ex:confidence ?conf .
            ?obj rdfs:label ?label .
            FILTER(xsd:float(?conf) > 0.9)
        } ORDER BY DESC(?conf) LIMIT 20
    """)
    print(f"\n  Query: High-confidence objects (>0.9) → {len(q2)} results (top 20)")
    for row in q2:
        print(f"    {row.label} ({row.conf})")

    # Query 3: Objects by category — find all animals
    q3 = g.query("""
        PREFIX schema: <http://schema.org/>
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        SELECT DISTINCT ?label ?cat WHERE {
            ?obj schema:category ?cat .
            ?obj rdfs:label ?label .
            FILTER(CONTAINS(LCASE(STR(?cat)), "animal"))
        }
    """)
    print(f"\n  Query: Find all animals → {len(q3)} results")
    for row in q3:
        print(f"    {row.label} [{row.cat}]")

    # Query 4: Spatial relationships
    q4 = g.query("""
        PREFIX ex: <http://example.org/image/>
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        SELECT ?subj_label ?rel ?obj_label WHERE {
            ?subj ex:spatialRelation ?bn .
            ?bn rdfs:label ?rel .
            ?subj rdfs:label ?subj_label .
            OPTIONAL { ?bn ex:target ?target . ?target rdfs:label ?obj_label }
        } LIMIT 15
    """)
    print(f"\n  Query: Spatial relationships → {len(q4)} results (first 15)")
    for row in q4:
        target = row.obj_label if row.obj_label else "—"
        print(f"    {row.subj_label} --[{row.rel}]--> {target}")

    # Query 5: Images containing vehicles
    q5 = g.query("""
        PREFIX ex: <http://example.org/image/>
        PREFIX schema: <http://schema.org/>
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        SELECT DISTINCT ?img ?label WHERE {
            ?img a schema:ImageObject .
            ?img (ex:hasObject|ex:containsObject|schema:hasPart) ?obj .
            ?obj schema:category ?cat .
            ?obj rdfs:label ?label .
            FILTER(CONTAINS(LCASE(STR(?cat)), "vehicle"))
        }
    """)
    print(f"\n  Query: Images containing vehicles → {len(q5)} results")
    for row in q5:
        img_name = str(row.img).split("/")[-1]
        print(f"    {img_name}: {row.label}")

    # Query 6: Count synonyms (skos:altLabel coverage)
    q6 = g.query("""
        PREFIX skos: <http://www.w3.org/2004/02/skos/core#>
        SELECT (COUNT(?alt) AS ?total_synonyms) WHERE {
            ?obj skos:altLabel ?alt .
        }
    """)
    synonym_count = list(q6)[0][0]
    print(f"\n  Query: Total skos:altLabel synonyms → {synonym_count}")

    # Ground truth comparison with altLabel expansion
    print("\n--- Step 4: Ground truth comparison (with synonym expansion) ---")
    recall_scores = []
    for img_file in sorted(gt.keys()):
        gt_objs = set(o.lower() for o in gt[img_file]["objects"])
        img_num = img_file.replace("img_", "").replace(".jpg", "")
        # Get all labels + altLabels for this image's objects
        q = g.query(f"""
            PREFIX ex: <http://example.org/image/>
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            PREFIX skos: <http://www.w3.org/2004/02/skos/core#>
            SELECT ?label WHERE {{
                ex:img_{img_num} (ex:hasObject|ex:containsObject|schema:hasPart) ?obj .
                {{ ?obj rdfs:label ?label }} UNION {{ ?obj skos:altLabel ?label }}
            }}
        """)
        detected = {str(row.label).lower() for row in q}
        hits = sum(1 for g_obj in gt_objs
                   if any(g_obj in d or d in g_obj for d in detected))
        recall = hits / len(gt_objs) if gt_objs else 1.0
        recall_scores.append(recall)
        status = "PERFECT" if recall == 1.0 else f"{recall:.0%}"
        print(f"  {img_file}: {status} ({hits}/{len(gt_objs)} objects)")

    avg_recall = sum(recall_scores) / len(recall_scores)
    print(f"\n  Average object recall: {avg_recall:.0%}")

    # Save results
    results = {
        "pipeline": "onto_validate (CLI) → rdflib load → SPARQL query",
        "images_processed": len(validated),
        "images_valid": len(validated),
        "total_triples": total_triples,
        "synonym_count": int(synonym_count),
        "avg_object_recall": round(avg_recall, 2),
        "sparql_queries_run": 6,
        "queryable": True,
    }
    out_path = os.path.join(DATASET_DIR, "pipeline_results.json")
    with open(out_path, "w") as f:
        json.dump(results, f, indent=2)

    print(f"\n{'=' * 80}")
    print(f"Pipeline complete: {total_triples} triples, {int(synonym_count)} synonyms, {avg_recall:.0%} recall")
    print(f"Results saved to {out_path}")


if __name__ == "__main__":
    main()
