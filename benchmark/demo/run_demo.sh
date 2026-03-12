#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OO="${OO_BIN:-open-ontologies}"

echo "=== SQL → OWL Demo ==="
echo ""

# Start postgres
echo "Starting PostgreSQL..."
cd "$SCRIPT_DIR"
docker compose up -d
echo "Waiting for postgres to be ready..."
for i in $(seq 1 30); do
    if docker compose exec -T postgres pg_isready -U demo -d shop > /dev/null 2>&1; then
        break
    fi
    sleep 1
done
echo ""

START=$(python3 -c "import time; print(int(time.time()*1000))")

echo "Step 1: Import schema..."
$OO import-schema "postgres://demo:demo@localhost:5433/shop" --base-iri "http://shop.example.org/" --pretty
echo ""

echo "Step 2: Classify..."
$OO reason --profile owl-dl --pretty
echo ""

echo "Step 3: Query classes..."
$OO query "SELECT ?c ?label WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> . OPTIONAL { ?c <http://www.w3.org/2000/01/rdf-schema#label> ?label } }" --pretty
echo ""

echo "Step 4: Stats..."
$OO stats --pretty

END=$(python3 -c "import time; print(int(time.time()*1000))")
ELAPSED=$(( END - START ))
echo ""
echo "Total pipeline time: ${ELAPSED}ms"

# Cleanup
echo ""
echo "Cleaning up..."
docker compose down
echo "Done."
