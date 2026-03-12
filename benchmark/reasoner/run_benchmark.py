#!/usr/bin/env python3
"""
Run Pizza and LUBM benchmarks comparing Open Ontologies vs HermiT.

Open Ontologies uses an in-memory store per CLI invocation, so we measure
the full load+reason cycle for fair comparison. HermiT also loads the
ontology fresh each invocation.
"""
import json
import os
import subprocess
import sys
import time

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
BENCHMARK_DIR = os.path.dirname(SCRIPT_DIR)
RESULTS_DIR = os.path.join(SCRIPT_DIR, "results")
os.makedirs(RESULTS_DIR, exist_ok=True)

OO_BIN = os.environ.get("OO_BIN", "open-ontologies")
JAVA = os.environ.get("JAVA", "java")
PIZZA_OWL = os.path.join(BENCHMARK_DIR, "reference", "pizza-reference.owl")


def run_oo_reason(owl_path, profile="owl-rl"):
    """Run Open Ontologies: load + reason in sequence (same data-dir)."""
    import tempfile
    data_dir = tempfile.mkdtemp()

    start = time.time()
    # Load
    load_result = subprocess.run(
        [OO_BIN, "load", owl_path, "--data-dir", data_dir],
        capture_output=True, text=True
    )
    load_data = json.loads(load_result.stdout) if load_result.stdout else {}

    # Reason
    reason_result = subprocess.run(
        [OO_BIN, "reason", "--profile", profile, "--data-dir", data_dir],
        capture_output=True, text=True
    )
    elapsed_ms = int((time.time() - start) * 1000)

    reason_data = json.loads(reason_result.stdout) if reason_result.stdout else {}
    reason_data["total_time_ms"] = elapsed_ms
    reason_data["triples_loaded"] = load_data.get("triples_loaded", 0)

    # Clean up
    import shutil
    shutil.rmtree(data_dir, ignore_errors=True)

    return reason_data


def run_hermit(owl_path, output_path):
    """Run HermiT via JavaReasoner."""
    cp = f"{SCRIPT_DIR}:{SCRIPT_DIR}/lib/*"
    result = subprocess.run(
        [JAVA, "-cp", cp, "JavaReasoner", "hermit", owl_path, output_path],
        capture_output=True, text=True
    )
    if result.returncode != 0:
        print(f"  HermiT error: {result.stderr[:200]}", file=sys.stderr)
        return None
    with open(output_path) as f:
        return json.load(f)


def pizza_benchmark():
    """Compare Pizza ontology classification: HermiT vs Open Ontologies."""
    print("=" * 60)
    print("Pizza Ontology Correctness Benchmark")
    print("=" * 60)
    print()

    if not os.path.exists(PIZZA_OWL):
        print(f"ERROR: {PIZZA_OWL} not found")
        return None

    # HermiT
    print("Running HermiT...")
    hermit_path = os.path.join(RESULTS_DIR, "hermit_pizza.json")
    hermit = run_hermit(PIZZA_OWL, hermit_path)
    if hermit:
        print(f"  HermiT: {hermit['time_ms']}ms, {hermit['classes']} classes, {len(hermit['subsumptions'])} subsumptions")
    else:
        print("  HermiT: FAILED")

    # Open Ontologies (owl-rl is the closest to classification)
    print("Running Open Ontologies (owl-rl)...")
    oo = run_oo_reason(PIZZA_OWL, "owl-rl")
    print(f"  OO: {oo['total_time_ms']}ms, {oo.get('triples_loaded', 0)} triples loaded, "
          f"{oo.get('inferred_count', 0)} inferred")

    # Also run owl-dl
    print("Running Open Ontologies (owl-dl)...")
    oo_dl = run_oo_reason(PIZZA_OWL, "owl-dl")
    print(f"  OO (owl-dl): {oo_dl['total_time_ms']}ms, "
          f"{oo_dl.get('inferred_subsumptions', 0)} subsumptions, "
          f"{oo_dl.get('named_classes', 0)} named classes")

    result = {
        "ontology": "Pizza",
        "source": "Manchester University reference (pizza-reference.owl)",
        "triples": oo.get("triples_loaded", 0),
        "hermit": {
            "time_ms": hermit["time_ms"] if hermit else None,
            "classes": hermit["classes"] if hermit else None,
            "subsumptions": len(hermit["subsumptions"]) if hermit else None,
        },
        "open_ontologies_owl_rl": {
            "time_ms": oo["total_time_ms"],
            "inferred_triples": oo.get("inferred_count", 0),
        },
        "open_ontologies_owl_dl": {
            "time_ms": oo_dl["total_time_ms"],
            "inferred_subsumptions": oo_dl.get("inferred_subsumptions", 0),
            "named_classes": oo_dl.get("named_classes", 0),
            "consistent": oo_dl.get("consistent", None),
        },
    }

    out_path = os.path.join(RESULTS_DIR, "pizza_comparison.json")
    with open(out_path, "w") as f:
        json.dump(result, f, indent=2)
    print(f"\nResults saved to {out_path}")

    return result


def lubm_benchmark():
    """Performance benchmark at increasing ontology sizes."""
    print()
    print("=" * 60)
    print("LUBM Performance Benchmark")
    print("=" * 60)
    print()

    sizes = [1000, 5000, 10000, 50000]

    # Generate LUBM ontologies
    print("Generating LUBM ontologies...")
    gen_script = os.path.join(SCRIPT_DIR, "generate_lubm.py")
    subprocess.run([sys.executable, gen_script] + [str(s) for s in sizes], check=True)
    print()

    results = []
    for size in sizes:
        owl_path = os.path.join(RESULTS_DIR, f"lubm_{size}.owl")
        if not os.path.exists(owl_path):
            print(f"SKIP: {owl_path} not found")
            continue

        print(f"--- Size: {size} axioms ---")

        # Open Ontologies
        oo = run_oo_reason(owl_path, "owl-rl")
        print(f"  OO: {oo['total_time_ms']}ms")

        # HermiT
        hermit_out = os.path.join(RESULTS_DIR, f"hermit_lubm_{size}.json")
        hermit = run_hermit(owl_path, hermit_out)
        hermit_ms = hermit["time_ms"] if hermit else None
        if hermit:
            print(f"  HermiT: {hermit_ms}ms")
        else:
            print(f"  HermiT: FAILED")

        results.append({
            "size": size,
            "oo_ms": oo["total_time_ms"],
            "hermit_ms": hermit_ms,
        })
        print()

    out_path = os.path.join(RESULTS_DIR, "lubm_results.json")
    with open(out_path, "w") as f:
        json.dump(results, f, indent=2)
    print(f"Results saved to {out_path}")

    return results


def print_summary(pizza, lubm):
    """Print a summary table."""
    print()
    print("=" * 60)
    print("BENCHMARK SUMMARY")
    print("=" * 60)
    print()

    if pizza:
        h = pizza["hermit"]
        oo = pizza["open_ontologies_owl_rl"]
        oo_dl = pizza["open_ontologies_owl_dl"]
        print(f"Pizza Ontology ({pizza['triples']} triples)")
        print(f"  {'Tool':<25} {'Time (ms)':<12} {'Result'}")
        print(f"  {'-'*55}")
        if h["time_ms"]:
            print(f"  {'HermiT':<25} {h['time_ms']:<12} {h['subsumptions']} subsumptions")
        print(f"  {'Open Ontologies (OWL-RL)':<25} {oo['time_ms']:<12} {oo['inferred_triples']} inferred triples")
        print(f"  {'Open Ontologies (OWL-DL)':<25} {oo_dl['time_ms']:<12} {oo_dl['inferred_subsumptions']} subsumptions, consistent={oo_dl['consistent']}")
        print()

    if lubm:
        print(f"LUBM Scaling")
        print(f"  {'Size':<10} {'OO (ms)':<12} {'HermiT (ms)':<12} {'Ratio'}")
        print(f"  {'-'*50}")
        for r in lubm:
            ratio = ""
            if r["hermit_ms"] and r["oo_ms"]:
                ratio = f"{r['hermit_ms']/r['oo_ms']:.1f}x"
            print(f"  {r['size']:<10} {r['oo_ms']:<12} {str(r['hermit_ms'] or 'N/A'):<12} {ratio}")


if __name__ == "__main__":
    pizza = pizza_benchmark()
    lubm = lubm_benchmark()
    print_summary(pizza, lubm)
