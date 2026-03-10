"""
Pizza Ontology Extension: Reasoning Benchmark
==============================================

Demonstrates the data extension pipeline and tests reasoning accuracy:
1. Load the AI-generated Pizza ontology (TBox)
2. Ingest restaurant menu CSV data (ABox) using the mapping config
3. Run RDFS reasoning to propagate subclass relationships
4. Classify vegetarian pizzas from the topping hierarchy
5. Compare against ground truth from the Manchester reference

Requirements: pip install rdflib
"""

import csv
from pathlib import Path
from rdflib import Graph, Namespace, RDF, RDFS, OWL, URIRef, BNode, Literal, XSD

PIZZA = Namespace("http://www.co-ode.org/ontologies/pizza/pizza.owl#")
REF_NS = Namespace("https://raw.githubusercontent.com/owlcs/pizza-ontology/refs/heads/master/pizza.owl#")
BASE = Path(__file__).parent


def load_ontology():
    """Load the AI-generated Pizza ontology (TBox)."""
    g = Graph()
    g.parse(str(BASE / "generated" / "pizza-ai.ttl"), format="turtle")
    return g


def load_reference_vegetarian():
    """Get vegetarian classification from the Manchester reference.
    A pizza is vegetarian if none of its toppings are Meat or Fish."""
    ref = Graph()
    ref.parse(str(BASE / "reference" / "pizza-reference.owl"), format="xml")

    # Get meat/fish topping classes
    meat = {REF_NS.MeatTopping}
    fish = {REF_NS.FishTopping}
    for sub in ref.subjects(RDFS.subClassOf, REF_NS.MeatTopping):
        meat.add(sub)
    for sub in ref.subjects(RDFS.subClassOf, REF_NS.FishTopping):
        fish.add(sub)

    veg_map = {}
    for pizza_uri in ref.subjects(RDFS.subClassOf, REF_NS.NamedPizza):
        name = str(pizza_uri).split("#")[-1]
        if name == "UnclosedPizza":
            continue

        toppings = set()
        for restriction in ref.objects(pizza_uri, RDFS.subClassOf):
            if not isinstance(restriction, BNode):
                continue
            on_prop = list(ref.objects(restriction, OWL.onProperty))
            some_val = list(ref.objects(restriction, OWL.someValuesFrom))
            if on_prop and some_val:
                prop = str(on_prop[0]).split("#")[-1]
                val = some_val[0]
                if prop == "hasTopping" and not isinstance(val, BNode):
                    toppings.add(val)

        is_veg = not any(t in meat or t in fish for t in toppings)
        veg_map[name] = is_veg

    return veg_map


def ingest_csv(g):
    """Ingest pizza-menu.csv into the graph, mapping toppings to ontology IRIs."""
    with open(BASE / "data" / "pizza-menu.csv") as f:
        rows = list(csv.DictReader(f))

    # Build topping name → class IRI lookup from ontology
    topping_lookup = {}
    for cls in g.subjects(RDF.type, OWL.Class):
        local = str(cls).split("#")[-1]
        if local.endswith("Topping"):
            short = local.replace("Topping", "").lower()
            topping_lookup[short] = cls
        else:
            # Handle PineKernels (no "Topping" suffix in ontology)
            topping_lookup[local.lower()] = cls

    loaded = 0
    for row in rows:
        name = row["name"].strip()
        if not name:
            continue
        subject = PIZZA[name]
        g.add((subject, RDF.type, PIZZA.NamedPizza))
        g.add((subject, RDFS.label, Literal(name, datatype=XSD.string)))

        for key in sorted(row.keys()):
            if key.startswith("topping") and row[key].strip():
                raw = row[key].strip()
                # Try to resolve to ontology class
                lookup_key = raw.lower()
                if lookup_key in topping_lookup:
                    topping_iri = topping_lookup[lookup_key]
                else:
                    # Fallback: construct IRI with Topping suffix
                    topping_iri = PIZZA[raw + "Topping"]
                g.add((subject, PIZZA.hasTopping, topping_iri))
                loaded += 1

    return rows, loaded


def run_rdfs_reasoning(g):
    """Apply RDFS reasoning to propagate subclass types."""
    new_triples = 0
    changed = True
    iterations = 0
    while changed and iterations < 20:
        changed = False
        iterations += 1
        to_add = []
        for x, _, a in g.triples((None, RDF.type, None)):
            for b in g.objects(a, RDFS.subClassOf):
                if (x, RDF.type, b) not in g:
                    to_add.append((x, RDF.type, b))
        for a, _, b in g.triples((None, RDFS.subClassOf, None)):
            for c in g.objects(b, RDFS.subClassOf):
                if (a, RDFS.subClassOf, c) not in g:
                    to_add.append((a, RDFS.subClassOf, c))
        if to_add:
            changed = True
            for t in to_add:
                g.add(t)
                new_triples += 1
    return new_triples, iterations


def classify_vegetarian(g):
    """Classify pizzas as vegetarian using the materialised type hierarchy."""
    meat_classes = {PIZZA.MeatTopping}
    fish_classes = {PIZZA.FishTopping}
    for sub in g.subjects(RDFS.subClassOf, PIZZA.MeatTopping):
        meat_classes.add(sub)
    for sub in g.subjects(RDFS.subClassOf, PIZZA.FishTopping):
        fish_classes.add(sub)

    results = {}
    for pizza_inst in g.subjects(RDF.type, PIZZA.NamedPizza):
        name = str(pizza_inst).split("#")[-1]
        toppings = list(g.objects(pizza_inst, PIZZA.hasTopping))

        is_veg = True
        for t in toppings:
            t_types = set(g.objects(t, RDF.type))
            # If the topping IRI is itself a class in the hierarchy, check directly
            if t in meat_classes or t in fish_classes:
                is_veg = False
                break
            # Also check if any of its types are meat/fish
            if t_types & meat_classes or t_types & fish_classes:
                is_veg = False
                break

        results[name] = is_veg

    return results


def main():
    print("=" * 70)
    print("Pizza Extension: Reasoning Benchmark")
    print("=" * 70)

    # Load TBox
    print("\n1. Loading AI-generated Pizza ontology (TBox)...")
    g = load_ontology()
    tbox_triples = len(g)
    print(f"   {tbox_triples} triples")

    # Ingest CSV
    print("\n2. Ingesting pizza-menu.csv...")
    rows, triples_loaded = ingest_csv(g)
    abox_triples = len(g) - tbox_triples
    print(f"   {len(rows)} rows → {abox_triples} ABox triples")

    # Reason
    print("\n3. Running RDFS reasoning...")
    new_triples, iterations = run_rdfs_reasoning(g)
    print(f"   {new_triples} inferred triples in {iterations} iterations")

    # Classify
    print("\n4. Classifying vegetarian pizzas...")
    inferred = classify_vegetarian(g)

    # Ground truth from Manchester reference
    ref_veg = load_reference_vegetarian()

    print(f"\n{'Pizza':<22} {'Reference':<15} {'Inferred':<15} {'Match':>5}")
    print("-" * 62)

    correct = 0
    total = 0
    for name in sorted(inferred.keys()):
        if name not in ref_veg:
            continue
        truth = ref_veg[name]
        inf = inferred[name]
        match = truth == inf
        correct += 1 if match else 0
        total += 1

        truth_str = "Vegetarian" if truth else "Non-veg"
        inf_str = "Vegetarian" if inf else "Non-veg"
        match_str = "YES" if match else "NO"
        print(f"{name:<22} {truth_str:<15} {inf_str:<15} {match_str:>5}")

    accuracy = correct / total * 100 if total else 0
    print("-" * 62)
    print(f"\nClassification accuracy: {correct}/{total} ({accuracy:.0f}%)")

    print(f"""
Summary
  TBox triples:       {tbox_triples}
  ABox triples:       {abox_triples}
  Inferred triples:   {new_triples}
  Reasoning accuracy: {correct}/{total} ({accuracy:.0f}%)
  Ground truth:       Manchester reference OWL (handcrafted)""")


if __name__ == "__main__":
    main()
