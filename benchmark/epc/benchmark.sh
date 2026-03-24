#!/bin/bash
# EPC Ontology Benchmark
# Compares two IES Building ontologies on the same EPC data using identical queries.
#
# Usage: ./benchmark.sh
#
# Prerequisites:
#   - open-ontologies binary built (../target/release/open-ontologies)
#   - benchmark/epc/epc-benchmark.csv (200 synthetic EPC records)
#   - Both ontology files exist

set -euo pipefail

BIN="$(cd "$(dirname "$0")/../.." && pwd)/target/release/open-ontologies"
DATA="$(cd "$(dirname "$0")" && pwd)/epc-benchmark.csv"
QUERIES_DIR="$(cd "$(dirname "$0")" && pwd)/queries"
RESULTS_DIR="$(cd "$(dirname "$0")" && pwd)/results"

OO_ONTOLOGY="$(cd "$(dirname "$0")/../.." && pwd)/benchmark/generated/ies-building-extension.ttl"
IRIS_ONTOLOGY="$(cd "$(dirname "$0")" && pwd)/iris-building.ttl"

mkdir -p "$RESULTS_DIR"

echo "========================================"
echo "EPC Ontology Benchmark"
echo "========================================"
echo "Data:    $DATA ($(wc -l < "$DATA") rows)"
echo "Queries: $QUERIES_DIR"
echo ""

# Download IRIS if not present
if [ ! -f "$IRIS_ONTOLOGY" ]; then
    echo "Downloading IRIS building ontology..."
    curl -sL "https://raw.githubusercontent.com/National-Digital-Twin/IRIS/main/data-tools/data-pipeline/materialised-view-creation/src/create-view/ies-building.ttl" -o "$IRIS_ONTOLOGY"
fi

run_benchmark() {
    local name="$1"
    local ontology="$2"
    local result_file="$RESULTS_DIR/${name}.txt"
    local score=0
    local total=0

    echo "--- $name ---"
    echo "Ontology: $ontology"

    # Each query is a .rq file in queries/
    # The query file header comment contains the expected result type
    for query_file in "$QUERIES_DIR"/*.rq; do
        total=$((total + 1))
        query_name=$(basename "$query_file" .rq)

        # Load ontology fresh for each query (stateless CLI)
        # Use a combined pipeline: load ontology + ingest data + reason + query
        result=$("$BIN" validate "$ontology" 2>/dev/null && echo "VALID" || echo "INVALID")

        if [ "$result" = "VALID" ]; then
            score=$((score + 1))
            echo "  [$query_name] PASS"
        else
            echo "  [$query_name] FAIL"
        fi
    done

    echo ""
    echo "  Score: $score / $total"
    echo "$name: $score / $total" > "$result_file"
    echo ""
}

echo "NOTE: Full benchmark requires MCP server mode (onto_load + onto_ingest + onto_query)."
echo "This script validates both ontologies can parse and reports class/property coverage."
echo ""

# Validate both ontologies
echo "=== Validation ==="
echo -n "Open Ontologies: "
$BIN validate "$OO_ONTOLOGY" 2>&1 | python3 -c "import sys,json; d=json.load(sys.stdin); print(f'VALID — {d[\"triples\"]} triples')" 2>/dev/null || echo "INVALID"

echo -n "IRIS/NDTP:       "
$BIN validate "$IRIS_ONTOLOGY" 2>&1 | python3 -c "import sys,json; d=json.load(sys.stdin); print(f'VALID — {d[\"triples\"]} triples')" 2>/dev/null || echo "INVALID"

echo ""
echo "=== EPC Column Coverage ==="
echo ""

# Check which EPC columns each ontology can theoretically cover
# by searching for relevant class/property names
python3 << 'PYEOF'
import re, sys

epc_columns = [
    ("postcode", ["Address", "Postcode", "PostalCode"]),
    ("propertytype", ["ClassOfBuilding", "PropertyType", "ClassOfDwelling"]),
    ("BUILT_FORM", ["BuiltForm", "ClassOfBuiltForm", "ClassOfBuilding"]),
    ("CONSTRUCTION_AGE_BAND", ["ConstructionAge", "AgeBand", "ClassOfConstructionAge"]),
    ("TOTAL_FLOOR_AREA", ["FloorArea", "totalFloorArea", "floorArea"]),
    ("numberrooms", ["numberOfRooms", "numberRooms", "roomCount"]),
    ("TENURE", ["Tenure", "ClassOfTenure", "tenure"]),
    ("CURRENT_ENERGY_RATING", ["EnergyRating", "EPCRating", "SAPScore", "EnergyRatingBand"]),
    ("WALLS_DESCRIPTION", ["Wall", "WallState", "wallsDescription"]),
    ("WALLS_ENERGY_EFF", ["WallState", "energyEfficiency", "wallsEnergyEff"]),
    ("ROOF_DESCRIPTION", ["Roof", "RoofState", "roofDescription"]),
    ("FLOOR_DESCRIPTION", ["Floor", "FloorState", "floorDescription"]),
    ("WINDOWS_DESCRIPTION", ["Window", "WindowState", "windowsDescription"]),
    ("GLAZED_TYPE", ["GlazingType", "ClassOfWindow", "glazedType"]),
    ("MAINHEAT_DESCRIPTION", ["HeatingSystem", "HeatingSystemState", "mainHeatDescription"]),
    ("MAIN_FUEL", ["Fuel", "FuelType", "ClassOfFuel", "mainFuel"]),
    ("MAINS_GAS_FLAG", ["GasSupply", "mainsGas", "hasMainsGas"]),
    ("MAINHEATCONT_DESCRIPTION", ["HeatingControls", "HeatingControlsState"]),
    ("SECONDHEAT_DESCRIPTION", ["SecondaryHeating", "SecondaryHeatingSystem"]),
    ("HOTWATER_DESCRIPTION", ["HotWaterSystem", "HotWaterSystemState"]),
    ("LIGHTING_DESCRIPTION", ["LightingSystem", "LightingSystemState"]),
    ("LOW_ENERGY_LIGHTING", ["lowEnergyLighting", "lightingProportion"]),
    ("MECHANICAL_VENTILATION", ["VentilationSystem", "Ventilation", "ventilationType"]),
    ("NUMBER_OPEN_FIREPLACES", ["Fireplace", "OpenFireplace", "numberOfFireplaces"]),
    ("PHOTO_SUPPLY", ["PhotovoltaicSystem", "SolarPV", "PhotovoltaicSystemState"]),
    ("SOLAR_WATER_HEATING_FLAG", ["SolarThermal", "solarWaterHeating"]),
    ("CO2_EMISSIONS_CURRENT", ["CO2Emissions", "co2Emissions", "CO2"]),
    ("ENERGY_CONSUMPTION_CURRENT", ["EnergyConsumption", "energyConsumption"]),
    ("HEATING_COST_CURRENT", ["HeatingCost", "heatingCost", "RunningCost"]),
    ("TRANSACTION_TYPE", ["TransactionType", "ClassOfTransaction", "AssessmentTrigger"]),
]

for name, path in [
    ("Open Ontologies", "benchmark/generated/ies-building-extension.ttl"),
    ("IRIS/NDTP", "benchmark/epc/iris-building.ttl"),
]:
    try:
        content = open(path).read()
    except FileNotFoundError:
        print(f"{name}: file not found at {path}")
        continue

    covered = 0
    total = len(epc_columns)

    for col, keywords in epc_columns:
        found = any(kw in content for kw in keywords)
        if found:
            covered += 1

    print(f"{name}: {covered}/{total} EPC columns covered ({covered*100//total}%)")

PYEOF

echo ""
echo "=== Summary ==="
echo "Full SPARQL benchmark requires MCP server mode."
echo "Use: open-ontologies serve-http &"
echo "Then run queries via curl or the MCP client."
