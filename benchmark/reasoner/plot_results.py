#!/usr/bin/env python3
"""Plot LUBM benchmark results as comparison chart."""
import json
import os
import sys

try:
    import matplotlib.pyplot as plt
    import matplotlib
    matplotlib.use("Agg")
except ImportError:
    print("matplotlib not installed — run: pip install matplotlib")
    sys.exit(1)


def main():
    results_dir = os.path.join(os.path.dirname(__file__), "results")
    results_file = os.path.join(results_dir, "lubm_results.json")

    if not os.path.exists(results_file):
        print(f"No results file found at {results_file}")
        print("Run run_lubm_performance.sh first.")
        sys.exit(1)

    with open(results_file) as f:
        data = json.load(f)

    sizes = [d["size"] for d in data]
    oo_times = [d["oo_ms"] for d in data]
    hermit_times = [d.get("hermit_ms") for d in data]
    pellet_times = [d.get("pellet_ms") for d in data]

    fig, ax = plt.subplots(figsize=(10, 6))

    x = range(len(sizes))
    width = 0.25

    ax.bar([i - width for i in x], oo_times, width, label="Open Ontologies", color="#2563eb")

    if any(t is not None and t != "null" for t in hermit_times):
        ht = [t if t is not None and t != "null" else 0 for t in hermit_times]
        ax.bar(x, ht, width, label="HermiT", color="#dc2626")

    if any(t is not None and t != "null" for t in pellet_times):
        pt = [t if t is not None and t != "null" else 0 for t in pellet_times]
        ax.bar([i + width for i in x], pt, width, label="Pellet", color="#16a34a")

    ax.set_xlabel("Ontology Size (axioms)")
    ax.set_ylabel("Classification Time (ms)")
    ax.set_title("LUBM Performance: Open Ontologies vs Java Reasoners")
    ax.set_xticks(x)
    ax.set_xticklabels([f"{s:,}" for s in sizes])
    ax.legend()
    ax.grid(axis="y", alpha=0.3)

    output_path = os.path.join(results_dir, "lubm_chart.png")
    plt.tight_layout()
    plt.savefig(output_path, dpi=150)
    print(f"Chart saved to {output_path}")


if __name__ == "__main__":
    main()
