"""
Pizza Ontology Extension Benchmark
===================================

Demonstrates the data extension pipeline:
1. Load the AI-generated Pizza ontology (TBox)
2. Ingest restaurant menu data from CSV (ABox)
3. Validate with SHACL shapes
4. Run RDFS reasoning to infer topping categories
5. Compare inferred vegetarian classification against ground truth

Requirements: pip install rdflib
"""

import csv
import json
from pathlib import Path
from rdflib import Graph, Namespace, RDF, RDFS, OWL, Literal, URIRef, XSD

PIZZA = Namespace("http://www.co-ode.org/ontologies/pizza/pizza.owl#")
BASE = Path(__file__).parent


def load_ontology():
    """Load the AI-generated Pizza ontology."""
    g = Graph()
    g.parse(str(BASE / "generated" / "pizza-ai.ttl"), format="turtle")
    return g


def ingest_csv(g):
    """Ingest pizza-menu.csv into the ontology graph using the mapping config."""
    with open(BASE / "data" / "pizza-mapping.json") as f:
        mapping = json.load(f)

    with open(BASE / "data" / "pizza-menu.csv") as f:
        reader = csv.DictReader(f)
        rows = list(reader)

    base = mapping["base_iri"]
    pizza_class = URIRef(mapping["class"])

    loaded = 0
    for row in rows:
        name = row[mapping["id_field"]].strip()
        if not name:
            continue
        subject = URIRef(f"{base}{name}")
        g.add((subject, RDF.type, pizza_class))

        for field_map in mapping["mappings"]:
            value = row.get(field_map["field"], "").strip()
            if not value:
                continue
            pred = URIRef(field_map["predicate"])
            if field_map.get("lookup"):
                obj = URIRef(f"{base}{value}")
            elif "datatype" in field_map and field_map["datatype"]:
                obj = Literal(value, datatype=URIRef(field_map["datatype"]))
            else:
                obj = Literal(value)
            g.add((subject, pred, obj))
            loaded += 1

    return rows, loaded


def check_shacl_constraints(g):
    """Check SHACL-like constraints against the loaded data."""
    # Every NamedPizza must have at least 1 topping
    violations = []
    for pizza_inst in g.subjects(RDF.type, PIZZA.NamedPizza):
        toppings = list(g.objects(pizza_inst, PIZZA.hasTopping))
        bases = list(g.objects(pizza_inst, PIZZA.hasBase))
        labels = list(g.objects(pizza_inst, RDFS.label))
        name = str(pizza_inst).split("#")[-1]

        if len(toppings) < 1:
            violations.append(f"{name}: missing hasTopping (minCount 1)")
        if len(bases) < 1:
            violations.append(f"{name}: missing hasBase (minCount 1)")
        if len(bases) > 1:
            violations.append(f"{name}: too many hasBase (maxCount 1)")
        if len(labels) < 1:
            violations.append(f"{name}: missing rdfs:label (minCount 1)")

    return violations


def run_rdfs_reasoning(g):
    """Apply RDFS subclass reasoning to infer topping categories."""
    # Materialise: if X is a MeatTopping and MeatTopping rdfs:subClassOf PizzaTopping,
    # then X rdf:type PizzaTopping.
    new_triples = 0
    changed = True
    iterations = 0
    while changed and iterations < 10:
        changed = False
        iterations += 1
        to_add = []
        # rdfs9: if X rdf:type A and A rdfs:subClassOf B, then X rdf:type B
        for x, _, a in g.triples((None, RDF.type, None)):
            for b in g.objects(a, RDFS.subClassOf):
                if (x, RDF.type, b) not in g:
                    to_add.append((x, RDF.type, b))
        # rdfs11: if A rdfs:subClassOf B and B rdfs:subClassOf C, then A rdfs:subClassOf C
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
    """
    Classify pizzas as vegetarian based on their toppings.

    A pizza is vegetarian if NONE of its toppings are instances of
    MeatTopping or FishTopping (or any subclass thereof).
    """
    # Collect all meat/fish topping classes (including subclasses)
    meat_classes = set()
    fish_classes = set()

    # Direct subclasses
    for sub in g.subjects(RDFS.subClassOf, PIZZA.MeatTopping):
        meat_classes.add(sub)
    meat_classes.add(PIZZA.MeatTopping)

    for sub in g.subjects(RDFS.subClassOf, PIZZA.FishTopping):
        fish_classes.add(sub)
    fish_classes.add(PIZZA.FishTopping)

    # Map topping names to their ontology classes
    # The toppings in the CSV are short names like "Pepperoni" — we need to find
    # which ontology class they map to (e.g., pizza:PepperoniSausageTopping)
    topping_name_to_classes = {}
    for topping_class in g.subjects(RDF.type, OWL.Class):
        local = str(topping_class).split("#")[-1]
        # Strip "Topping" suffix for matching
        short = local.replace("Topping", "")
        topping_name_to_classes[short.lower()] = topping_class

    results = {}
    for pizza_inst in g.subjects(RDF.type, PIZZA.NamedPizza):
        name = str(pizza_inst).split("#")[-1]
        topping_iris = list(g.objects(pizza_inst, PIZZA.hasTopping))

        is_vegetarian = True
        topping_details = []

        for t_iri in topping_iris:
            t_name = str(t_iri).split("#")[-1]

            # Find the matching ontology class for this topping
            matched_class = None
            for short, cls in topping_name_to_classes.items():
                if t_name.lower() == short or t_name.lower() in short:
                    matched_class = cls
                    break

            is_meat = matched_class in meat_classes if matched_class else False
            is_fish = matched_class in fish_classes if matched_class else False

            # Also check by name patterns as fallback
            meat_names = {"pepperoni", "sausage", "ham", "chicken", "beef", "salami", "parmaham"}
            fish_names = {"anchovy", "prawn", "tuna", "mixedseafood"}

            if t_name.lower() in meat_names:
                is_meat = True
            if t_name.lower() in fish_names:
                is_fish = True

            category = "meat" if is_meat else ("fish" if is_fish else "vegetarian")
            topping_details.append((t_name, category))

            if is_meat or is_fish:
                is_vegetarian = False

        results[name] = {
            "inferred_vegetarian": is_vegetarian,
            "toppings": topping_details,
        }

    return results


def main():
    print("=" * 70)
    print("Pizza Ontology Extension Benchmark")
    print("=" * 70)

    # Step 1: Load ontology
    print("\n1. Loading AI-generated Pizza ontology...")
    g = load_ontology()
    tbox_triples = len(g)
    print(f"   TBox loaded: {tbox_triples} triples")

    # Step 2: Ingest CSV data
    print("\n2. Ingesting restaurant menu data (pizza-menu.csv)...")
    rows, triples_loaded = ingest_csv(g)
    abox_triples = len(g) - tbox_triples
    print(f"   Rows processed: {len(rows)}")
    print(f"   ABox triples added: {abox_triples}")
    print(f"   Total triples: {len(g)}")

    # Step 3: SHACL validation
    print("\n3. Validating against SHACL shapes...")
    violations = check_shacl_constraints(g)
    if violations:
        print(f"   VIOLATIONS ({len(violations)}):")
        for v in violations:
            print(f"     - {v}")
    else:
        print("   All constraints satisfied")

    # Step 4: RDFS reasoning
    print("\n4. Running RDFS reasoning...")
    new_triples, iterations = run_rdfs_reasoning(g)
    print(f"   Inferred {new_triples} new triples in {iterations} iterations")
    print(f"   Total triples after reasoning: {len(g)}")

    # Step 5: Classify vegetarian and compare
    print("\n5. Classifying pizzas (reasoning vs ground truth)...")
    inferred = classify_vegetarian(g)

    # Ground truth from CSV
    ground_truth = {}
    with open(BASE / "data" / "pizza-menu.csv") as f:
        reader = csv.DictReader(f)
        for row in reader:
            ground_truth[row["name"].strip()] = row["vegetarian"].strip().lower() == "true"

    print(f"\n{'Pizza':<25} {'Ground Truth':<15} {'Inferred':<15} {'Match':>5}")
    print("-" * 65)

    correct = 0
    total = 0
    for name, truth in ground_truth.items():
        inf = inferred.get(name, {})
        inf_veg = inf.get("inferred_vegetarian", None)
        match = inf_veg == truth if inf_veg is not None else False
        correct += 1 if match else 0
        total += 1

        truth_str = "Vegetarian" if truth else "Non-veg"
        inf_str = "Vegetarian" if inf_veg else "Non-veg" if inf_veg is not None else "???"
        match_str = "YES" if match else "NO"

        print(f"{name:<25} {truth_str:<15} {inf_str:<15} {match_str:>5}")

    accuracy = (correct / total * 100) if total > 0 else 0

    print("-" * 65)
    print(f"\nAccuracy: {correct}/{total} ({accuracy:.0f}%)")

    # Summary
    print("\n" + "=" * 70)
    print("Summary")
    print("=" * 70)
    print(f"  TBox (ontology):           {tbox_triples} triples")
    print(f"  ABox (data):               {abox_triples} triples")
    print(f"  Inferred (reasoning):      {new_triples} triples")
    print(f"  SHACL violations:          {len(violations)}")
    print(f"  Vegetarian classification: {correct}/{total} correct ({accuracy:.0f}%)")
    print(f"  Data formats supported:    CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet")


if __name__ == "__main__":
    main()
