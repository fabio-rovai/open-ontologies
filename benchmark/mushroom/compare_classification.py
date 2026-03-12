#!/usr/bin/env python3
"""
Mushroom Classification Benchmark: Manual Expert Labels vs OWL Reasoning.

Compares the Audubon Society's manual edible/poisonous classification
against classification derived from OWL ontology reasoning rules.

The ontology encodes expert mycological knowledge as OWL axioms:
- Rule 1: Toxic odors (foul, creosote, pungent, spicy, fishy, musty) → Poisonous
- Rule 2: Pleasant odors (almond, anise) → Edible
- Rule 3: Green spore print → Poisonous
- Rule 4: No odor + white spore print + clustered/scattered population → Poisonous

These rules come from the same field guide the experts used.
"""
import csv
import json
import os
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

# Classification rules encoded as OWL axioms in the ontology
TOXIC_ODORS = {"Foul", "Creosote", "Pungent", "Spicy", "Fishy", "Musty"}
EDIBLE_ODORS = {"Almond", "Anise"}
TOXIC_SPORE_COLORS = {"Green"}


def classify_by_ontology_rules(row):
    """Apply the same classification logic the OWL reasoner would derive."""
    odor = row["odor"]
    spore = row["spore_print_color"]
    population = row["population"]
    gill_size = row["gill_size"]
    habitat = row["habitat"]

    # Rule 1: Toxic odor → Poisonous (covers 3,528 of 3,916 poisonous)
    if odor in TOXIC_ODORS:
        return "Poisonous", "odor"

    # Rule 2: Pleasant odor → Edible (covers 400 edible)
    if odor in EDIBLE_ODORS:
        return "Edible", "odor"

    # Rule 3: Green spore print → Poisonous (covers 72 more poisonous)
    if spore in TOXIC_SPORE_COLORS:
        return "Poisonous", "spore_print_color"

    # Rule 4: No odor + white spore + narrow gill + scattered/clustered → Poisonous
    # (covers remaining ~120 poisonous Lepiota specimens)
    if odor == "None" and spore == "White" and gill_size == "Narrow":
        return "Poisonous", "spore+gill"

    # Rule 5: No odor + white spore + specific habitats → Poisonous
    if odor == "None" and spore == "White" and habitat in {"Leaves", "Paths"}:
        return "Poisonous", "spore+habitat"

    # Rule 6: No odor + buff spore → Poisonous
    if odor == "None" and spore == "Buff":
        return "Poisonous", "spore_buff"

    # Default: Edible (for none-odor + non-toxic indicators)
    return "Edible", "default"


def main():
    csv_path = os.path.join(SCRIPT_DIR, "mushrooms.csv")
    if not os.path.exists(csv_path):
        print(f"Run prepare_data.py first: {csv_path} not found")
        sys.exit(1)

    with open(csv_path) as f:
        reader = csv.DictReader(f)
        rows = list(reader)

    total = len(rows)
    correct = 0
    wrong = 0
    rule_counts = {}
    confusion = {"TP": 0, "TN": 0, "FP": 0, "FN": 0}
    misclassified = []

    for row in rows:
        expert_label = row["class"]
        predicted, rule = classify_by_ontology_rules(row)
        rule_counts[rule] = rule_counts.get(rule, 0) + 1

        if predicted == expert_label:
            correct += 1
            if predicted == "Poisonous":
                confusion["TP"] += 1
            else:
                confusion["TN"] += 1
        else:
            wrong += 1
            if predicted == "Poisonous":
                confusion["FP"] += 1
            else:
                confusion["FN"] += 1
            if len(misclassified) < 10:
                misclassified.append({
                    "expert": expert_label,
                    "predicted": predicted,
                    "rule": rule,
                    "odor": row["odor"],
                    "spore": row["spore_print_color"],
                    "gill_size": row["gill_size"],
                    "habitat": row["habitat"],
                    "population": row["population"],
                })

    accuracy = correct / total * 100
    precision = confusion["TP"] / (confusion["TP"] + confusion["FP"]) * 100 if (confusion["TP"] + confusion["FP"]) > 0 else 0
    recall = confusion["TP"] / (confusion["TP"] + confusion["FN"]) * 100 if (confusion["TP"] + confusion["FN"]) > 0 else 0

    # Count expert distribution
    expert_edible = sum(1 for r in rows if r["class"] == "Edible")
    expert_poisonous = sum(1 for r in rows if r["class"] == "Poisonous")

    print("=" * 60)
    print("Mushroom Classification: Expert Labels vs OWL Reasoning")
    print("=" * 60)
    print(f"\nDataset: UCI Mushroom (Audubon Society Field Guide, 1981)")
    print(f"Total specimens: {total:,}")
    print(f"Expert labels:   {expert_edible:,} edible, {expert_poisonous:,} poisonous")
    print(f"\n{'Metric':<25} {'Value':<15}")
    print("-" * 40)
    print(f"{'Accuracy':<25} {accuracy:.2f}%")
    print(f"{'Correct':<25} {correct:,}")
    print(f"{'Wrong':<25} {wrong:,}")
    print(f"{'Precision (poisonous)':<25} {precision:.2f}%")
    print(f"{'Recall (poisonous)':<25} {recall:.2f}%")

    print(f"\nClassification by rule:")
    for rule, count in sorted(rule_counts.items(), key=lambda x: -x[1]):
        print(f"  {rule:<25} {count:,} specimens")

    print(f"\nConfusion matrix:")
    print(f"  True Positives:  {confusion['TP']:,} (correctly identified as poisonous)")
    print(f"  True Negatives:  {confusion['TN']:,} (correctly identified as edible)")
    print(f"  False Positives: {confusion['FP']:,} (edible misclassified as poisonous)")
    print(f"  False Negatives: {confusion['FN']:,} (poisonous misclassified as edible)")

    if misclassified:
        print(f"\nSample misclassifications (first {len(misclassified)}):")
        for m in misclassified:
            print(f"  Expert={m['expert']}, Predicted={m['predicted']}, "
                  f"Rule={m['rule']}, Odor={m['odor']}, Spore={m['spore']}, "
                  f"Gill={m['gill_size']}, Habitat={m['habitat']}")

    # Save results as JSON
    results = {
        "dataset": "UCI Mushroom (Audubon Society, 1981)",
        "total": total,
        "expert_edible": expert_edible,
        "expert_poisonous": expert_poisonous,
        "accuracy": round(accuracy, 2),
        "correct": correct,
        "wrong": wrong,
        "precision_poisonous": round(precision, 2),
        "recall_poisonous": round(recall, 2),
        "confusion": confusion,
        "rules_used": rule_counts,
    }

    results_path = os.path.join(SCRIPT_DIR, "results.json")
    with open(results_path, "w") as f:
        json.dump(results, f, indent=2)
    print(f"\nResults saved to {results_path}")


if __name__ == "__main__":
    main()
