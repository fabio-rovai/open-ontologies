"""
Pizza Ontology Comparison: AI-generated vs Manchester Reference
Compares an AI-generated Pizza ontology (from Claude) against the canonical
Manchester University Pizza ontology used in the Protege OWL tutorial.
"""

from rdflib import Graph, RDF, RDFS, OWL, Namespace
import json

PIZZA = Namespace("http://www.co-ode.org/ontologies/pizza/pizza.owl#")


def load_graphs():
    ref = Graph()
    ref.parse("reference/pizza-reference.owl", format="xml")

    ai = Graph()
    ai.parse(
        "generated/pizza-ai.ttl",
        format="turtle",
    )
    return ref, ai


def get_classes(g):
    classes = set()
    for s in g.subjects(RDF.type, OWL.Class):
        local = str(s).split("#")[-1] if "#" in str(s) else None
        if local and not str(s).startswith("http://www.w3.org/"):
            classes.add(local)
    return classes


def get_properties(g):
    props = set()
    for s in g.subjects(RDF.type, OWL.ObjectProperty):
        local = str(s).split("#")[-1] if "#" in str(s) else None
        if local:
            props.add(local)
    for s in g.subjects(RDF.type, OWL.DatatypeProperty):
        local = str(s).split("#")[-1] if "#" in str(s) else None
        if local:
            props.add(local)
    return props


def get_subclass_pairs(g):
    pairs = set()
    for s, o in g.subject_objects(RDFS.subClassOf):
        s_local = str(s).split("#")[-1] if "#" in str(s) else None
        o_local = str(o).split("#")[-1] if "#" in str(o) else None
        if s_local and o_local and not str(s).startswith("http://www.w3.org/"):
            pairs.add((s_local, o_local))
    return pairs


def get_restrictions(g):
    restrictions = set()
    for s in g.subjects(RDF.type, OWL.Restriction):
        prop = None
        for p_val in g.objects(s, OWL.onProperty):
            prop = str(p_val).split("#")[-1] if "#" in str(p_val) else str(p_val)
        restriction_type = None
        value = None
        for v in g.objects(s, OWL.someValuesFrom):
            restriction_type = "some"
            value = str(v).split("#")[-1] if "#" in str(v) else str(v)
        for v in g.objects(s, OWL.allValuesFrom):
            restriction_type = "all"
            value = str(v).split("#")[-1] if "#" in str(v) else str(v)
        if prop and restriction_type:
            restrictions.add((prop, restriction_type, value or "?"))
    return restrictions


def get_disjoint_pairs(g):
    pairs = set()
    for s, o in g.subject_objects(OWL.disjointWith):
        s_local = str(s).split("#")[-1] if "#" in str(s) else None
        o_local = str(o).split("#")[-1] if "#" in str(o) else None
        if s_local and o_local:
            pair = tuple(sorted([s_local, o_local]))
            pairs.add(pair)
    return pairs


def get_labels(g):
    labels = set()
    for s, o in g.subject_objects(RDFS.label):
        local = str(s).split("#")[-1] if "#" in str(s) else None
        if local:
            labels.add(local)
    return labels


def categorize_classes(classes):
    cats = {
        "Toppings": set(),
        "Named Pizzas": set(),
        "Pizza Types": set(),
        "Bases": set(),
        "Spiciness": set(),
        "Other": set(),
    }
    named_pizzas = {
        "Margherita", "American", "AmericanHot", "Cajun", "Capricciosa",
        "Caprina", "Country", "Fiorentina", "FourSeasons", "FruttiDiMare",
        "Giardiniera", "LaReine", "Mushroom", "Napoletana", "Parmense",
        "PolloAdAstra", "PrinceCarlo", "QuattroFormaggi", "Rosa",
        "Siciliana", "SloppyGiuseppe", "Soho", "Veneziana", "NamedPizza",
    }
    pizza_types = {
        "Pizza", "CheeseyPizza", "InterestingPizza", "MeatyPizza",
        "NonVegetarianPizza", "RealItalianPizza", "SpicyPizza",
        "SpicyPizzaEquivalent", "ThinAndCrispyPizza", "UnclosedPizza",
        "VegetarianPizza", "VegetarianPizzaEquivalent1",
        "VegetarianPizzaEquivalent2",
    }

    for c in classes:
        if "Topping" in c or c in ("SpicyTopping", "VegetarianTopping", "CheeseyVegetableTopping"):
            cats["Toppings"].add(c)
        elif c in named_pizzas:
            cats["Named Pizzas"].add(c)
        elif c in pizza_types:
            cats["Pizza Types"].add(c)
        elif "Base" in c or c == "PizzaBase":
            cats["Bases"].add(c)
        elif c in ("Spiciness", "Hot", "Medium", "Mild"):
            cats["Spiciness"].add(c)
        else:
            cats["Other"].add(c)
    return cats


def main():
    ref, ai = load_graphs()

    ref_classes = get_classes(ref)
    ai_classes = get_classes(ai)
    ref_props = get_properties(ref)
    ai_props = get_properties(ai)
    ref_labels = get_labels(ref)
    ai_labels = get_labels(ai)
    ref_disjoints = get_disjoint_pairs(ref)
    ai_disjoints = get_disjoint_pairs(ai)

    shared_classes = ref_classes & ai_classes
    ref_only = ref_classes - ai_classes
    ai_only = ai_classes - ref_classes
    shared_props = ref_props & ai_props
    ref_only_props = ref_props - ai_props
    ai_only_props = ai_props - ref_props

    ref_cats = categorize_classes(ref_classes)
    ai_cats = categorize_classes(ai_classes)

    print("=" * 70)
    print("PIZZA ONTOLOGY COMPARISON")
    print("AI-Generated (Claude) vs Manchester Reference")
    print("=" * 70)

    print(f"\n{'Metric':<35} {'Reference':>12} {'AI-Generated':>14} {'Match':>8}")
    print("-" * 70)
    print(f"{'Total triples':<35} {len(ref):>12} {len(ai):>14} {len(ai)/len(ref)*100:>7.1f}%")
    print(f"{'Classes':<35} {len(ref_classes):>12} {len(ai_classes):>14} {len(shared_classes)/len(ref_classes)*100:>7.1f}%")
    print(f"{'Properties':<35} {len(ref_props):>12} {len(ai_props):>14} {len(shared_props)/len(ref_props)*100:>7.1f}%")
    print(f"{'Labels (rdfs:label)':<35} {len(ref_labels):>12} {len(ai_labels):>14}")
    print(f"{'Disjoint pairs':<35} {len(ref_disjoints):>12} {len(ai_disjoints):>14}")

    print(f"\n{'Category':<25} {'Reference':>10} {'AI':>10} {'Shared':>10} {'Coverage':>10}")
    print("-" * 70)
    for cat in ["Toppings", "Named Pizzas", "Pizza Types", "Bases", "Spiciness", "Other"]:
        r = ref_cats.get(cat, set())
        a = ai_cats.get(cat, set())
        s = r & a
        cov = f"{len(s)/len(r)*100:.0f}%" if r else "N/A"
        print(f"{cat:<25} {len(r):>10} {len(a):>10} {len(s):>10} {cov:>10}")

    print(f"\n--- Properties ---")
    print(f"Shared:       {sorted(shared_props)}")
    print(f"Ref only:     {sorted(ref_only_props)}")
    print(f"AI only:      {sorted(ai_only_props)}")

    print(f"\n--- Classes in reference but missing from AI ({len(ref_only)}) ---")
    for c in sorted(ref_only):
        print(f"  - {c}")

    print(f"\n--- Classes in AI but not in reference ({len(ai_only)}) ---")
    for c in sorted(ai_only):
        print(f"  - {c}")

    # Labels coverage
    print(f"\n--- Labels ---")
    print(f"AI has rdfs:label on {len(ai_labels)} classes")
    print(f"Reference has rdfs:label on {len(ref_labels)} classes")

    # Disjoint coverage
    shared_disjoints = ref_disjoints & ai_disjoints
    print(f"\n--- Disjointness ---")
    print(f"Reference disjoint pairs: {len(ref_disjoints)}")
    print(f"AI disjoint pairs:        {len(ai_disjoints)}")
    print(f"Shared disjoint pairs:    {len(shared_disjoints)}")
    print(f"Coverage:                 {len(shared_disjoints)/len(ref_disjoints)*100:.1f}%")

    # Overall score
    class_score = len(shared_classes) / len(ref_classes) * 100
    prop_score = len(shared_props) / len(ref_props) * 100
    disjoint_score = len(shared_disjoints) / len(ref_disjoints) * 100 if ref_disjoints else 0
    overall = (class_score * 0.4 + prop_score * 0.3 + disjoint_score * 0.3)

    print(f"\n{'=' * 70}")
    print(f"OVERALL SCORES")
    print(f"{'=' * 70}")
    print(f"  Class coverage:     {class_score:.1f}%  (weight: 40%)")
    print(f"  Property coverage:  {prop_score:.1f}%  (weight: 30%)")
    print(f"  Disjoint coverage:  {disjoint_score:.1f}%  (weight: 30%)")
    print(f"  ─────────────────────────────")
    print(f"  WEIGHTED SCORE:     {overall:.1f}%")
    print()
    print(f"  Triple efficiency:  {len(ref)}/{len(ai)} = {len(ref)/len(ai):.1f}x more verbose in reference")
    print(f"  AI approach:        {len(ai)} triples to cover {class_score:.0f}% of classes")

    # Save results as JSON
    results = {
        "reference": {
            "triples": len(ref),
            "classes": len(ref_classes),
            "properties": len(ref_props),
            "disjoint_pairs": len(ref_disjoints),
        },
        "ai_generated": {
            "triples": len(ai),
            "classes": len(ai_classes),
            "properties": len(ai_props),
            "disjoint_pairs": len(ai_disjoints),
        },
        "coverage": {
            "classes_pct": round(class_score, 1),
            "properties_pct": round(prop_score, 1),
            "disjoint_pct": round(disjoint_score, 1),
            "weighted_score": round(overall, 1),
        },
        "missing_classes": sorted(ref_only),
        "extra_classes": sorted(ai_only),
    }

    with open("pizza_results.json", "w") as f:
        json.dump(results, f, indent=2)
    print("\nResults saved to pizza_results.json")


if __name__ == "__main__":
    main()
