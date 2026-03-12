#!/usr/bin/env python3
"""Prepare mushroom dataset: add headers, expand codes to readable labels."""
import csv
import os

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

HEADERS = [
    "class", "cap_shape", "cap_surface", "cap_color", "bruises", "odor",
    "gill_attachment", "gill_spacing", "gill_size", "gill_color",
    "stalk_shape", "stalk_root", "stalk_surface_above_ring",
    "stalk_surface_below_ring", "stalk_color_above_ring",
    "stalk_color_below_ring", "veil_type", "veil_color",
    "ring_number", "ring_type", "spore_print_color", "population", "habitat"
]

# Map single-letter codes to readable labels for key columns
ODOR_MAP = {
    "a": "Almond", "l": "Anise", "c": "Creosote", "y": "Fishy",
    "f": "Foul", "m": "Musty", "n": "None", "p": "Pungent", "s": "Spicy"
}

SPORE_MAP = {
    "k": "Black", "n": "Brown", "b": "Buff", "h": "Chocolate",
    "r": "Green", "o": "Orange", "u": "Purple", "w": "White", "y": "Yellow"
}

CAP_COLOR_MAP = {
    "n": "Brown", "b": "Buff", "c": "Cinnamon", "g": "Gray", "r": "Green",
    "p": "Pink", "u": "Purple", "e": "Red", "w": "White", "y": "Yellow"
}

HABITAT_MAP = {
    "g": "Grasses", "l": "Leaves", "m": "Meadows", "p": "Paths",
    "u": "Urban", "w": "Waste", "d": "Woods"
}

POPULATION_MAP = {
    "a": "Abundant", "c": "Clustered", "n": "Numerous",
    "s": "Scattered", "v": "Several", "y": "Solitary"
}

CLASS_MAP = {"e": "Edible", "p": "Poisonous"}
BRUISES_MAP = {"t": "Yes", "f": "No"}
GILL_SIZE_MAP = {"b": "Broad", "n": "Narrow"}

def main():
    raw_path = os.path.join(SCRIPT_DIR, "raw.csv")
    out_path = os.path.join(SCRIPT_DIR, "mushrooms.csv")

    with open(raw_path) as f:
        reader = csv.reader(f)
        rows = list(reader)

    with open(out_path, "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(HEADERS)
        for row in rows:
            if len(row) != 23:
                continue
            row[0] = CLASS_MAP.get(row[0], row[0])
            row[5] = ODOR_MAP.get(row[5], row[5])
            row[20] = SPORE_MAP.get(row[20], row[20])
            row[3] = CAP_COLOR_MAP.get(row[3], row[3])
            row[22] = HABITAT_MAP.get(row[22], row[22])
            row[21] = POPULATION_MAP.get(row[21], row[21])
            row[4] = BRUISES_MAP.get(row[4], row[4])
            row[8] = GILL_SIZE_MAP.get(row[8], row[8])
            writer.writerow(row)

    print(f"Wrote {len(rows)} rows to {out_path}")


if __name__ == "__main__":
    main()
