#!/usr/bin/env bash
# Benchmark all 29 marketplace ontologies through the MCP pipeline
# Uses the HTTP server so state persists between tool calls
set -euo pipefail

BIN="./target/release/open-ontologies"
HOST="127.0.0.1"
PORT="9999"
MCP_URL="http://${HOST}:${PORT}/mcp"
RESULTS_FILE="benchmark/marketplace_results.json"

# Start server in background
echo "Starting server on port ${PORT}..."
$BIN serve-http --host $HOST --port $PORT &
SERVER_PID=$!
sleep 2

# MCP call helper — sends JSON-RPC and extracts result
MCP_ID=0
SESSION_ID=""

mcp_init() {
  local resp
  resp=$(curl -s -X POST "$MCP_URL" \
    -H "Content-Type: application/json" \
    -H "Accept: application/json, text/event-stream" \
    -d '{
      "jsonrpc": "2.0",
      "id": 1,
      "method": "initialize",
      "params": {
        "protocolVersion": "2025-03-26",
        "capabilities": {},
        "clientInfo": { "name": "benchmark", "version": "1.0.0" }
      }
    }')
  SESSION_ID=$(echo "$resp" | grep -o '"Mcp-Session-Id":"[^"]*"' | head -1 | cut -d'"' -f4 || true)
  if [ -z "$SESSION_ID" ]; then
    # Try extracting from headers via -i
    local headers
    headers=$(curl -s -i -X POST "$MCP_URL" \
      -H "Content-Type: application/json" \
      -H "Accept: application/json, text/event-stream" \
      -d '{
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
          "protocolVersion": "2025-03-26",
          "capabilities": {},
          "clientInfo": { "name": "benchmark", "version": "1.0.0" }
        }
      }')
    SESSION_ID=$(echo "$headers" | grep -i "mcp-session-id" | head -1 | tr -d '\r' | awk '{print $2}')
  fi
  echo "Session: $SESSION_ID"
}

mcp_call() {
  local tool_name="$1"
  local args="$2"
  MCP_ID=$((MCP_ID + 1))
  local payload
  payload=$(cat <<ENDJSON
{
  "jsonrpc": "2.0",
  "id": ${MCP_ID},
  "method": "tools/call",
  "params": {
    "name": "${tool_name}",
    "arguments": ${args}
  }
}
ENDJSON
)
  local result
  result=$(curl -s -X POST "$MCP_URL" \
    -H "Content-Type: application/json" \
    -H "Accept: application/json, text/event-stream" \
    ${SESSION_ID:+-H "Mcp-Session-Id: $SESSION_ID"} \
    -d "$payload")
  echo "$result"
}

cleanup() {
  echo "Stopping server..."
  kill $SERVER_PID 2>/dev/null || true
  wait $SERVER_PID 2>/dev/null || true
}
trap cleanup EXIT

# Initialize MCP session
mcp_init

# Get list of all ontology IDs
IDS=$(echo '["owl","rdfs","rdf","bfo","dolce","schema-org","foaf","skos","dc-elements","dc-terms","dcat","void","doap","prov-o","owl-time","org","ssn","sosa","geosparql","locn","shacl","vcard","odrl","cc","sioc","adms","goodrelations","fibo","qudt"]' | python3 -c "import json,sys; [print(x) for x in json.load(sys.stdin)]")

echo "["  > "$RESULTS_FILE"
FIRST=true

for ID in $IDS; do
  echo ""
  echo "=== Benchmarking: $ID ==="

  # Clear store
  mcp_call "onto_clear" '{}' > /dev/null

  # Install from marketplace (timed)
  START=$(python3 -c "import time; print(int(time.time()*1000))")
  INSTALL_RESULT=$(mcp_call "onto_marketplace" "{\"action\":\"install\",\"id\":\"${ID}\"}")
  END=$(python3 -c "import time; print(int(time.time()*1000))")
  FETCH_MS=$((END - START))

  # Extract triple count from install result
  TRIPLES_RAW=$(echo "$INSTALL_RESULT" | python3 -c "
import json, sys
try:
    data = json.load(sys.stdin)
    content = data.get('result', {}).get('content', [{}])
    if isinstance(content, list) and len(content) > 0:
        text = content[0].get('text', '{}')
        parsed = json.loads(text)
        print(parsed.get('triples_loaded', 0))
    else:
        print(0)
except:
    print(0)
" 2>/dev/null)

  # Get stats before reasoning
  STATS_BEFORE=$(mcp_call "onto_stats" '{}')
  CLASSES=$(echo "$STATS_BEFORE" | python3 -c "
import json, sys
try:
    data = json.load(sys.stdin)
    content = data.get('result', {}).get('content', [{}])
    text = content[0].get('text', '{}') if isinstance(content, list) and len(content) > 0 else '{}'
    parsed = json.loads(text)
    print(parsed.get('classes', 0))
except:
    print(0)
" 2>/dev/null)
  PROPERTIES=$(echo "$STATS_BEFORE" | python3 -c "
import json, sys
try:
    data = json.load(sys.stdin)
    content = data.get('result', {}).get('content', [{}])
    text = content[0].get('text', '{}') if isinstance(content, list) and len(content) > 0 else '{}'
    parsed = json.loads(text)
    print(parsed.get('properties', 0))
except:
    print(0)
" 2>/dev/null)
  TRIPLES_BEFORE=$(echo "$STATS_BEFORE" | python3 -c "
import json, sys
try:
    data = json.load(sys.stdin)
    content = data.get('result', {}).get('content', [{}])
    text = content[0].get('text', '{}') if isinstance(content, list) and len(content) > 0 else '{}'
    parsed = json.loads(text)
    print(parsed.get('triples', 0))
except:
    print(0)
" 2>/dev/null)

  # Reason (RDFS)
  START_R=$(python3 -c "import time; print(int(time.time()*1000))")
  mcp_call "onto_reason" '{"profile":"rdfs"}' > /dev/null
  END_R=$(python3 -c "import time; print(int(time.time()*1000))")
  REASON_MS=$((END_R - START_R))

  # Stats after reasoning
  STATS_AFTER=$(mcp_call "onto_stats" '{}')
  TRIPLES_AFTER=$(echo "$STATS_AFTER" | python3 -c "
import json, sys
try:
    data = json.load(sys.stdin)
    content = data.get('result', {}).get('content', [{}])
    text = content[0].get('text', '{}') if isinstance(content, list) and len(content) > 0 else '{}'
    parsed = json.loads(text)
    print(parsed.get('triples', 0))
except:
    print(0)
" 2>/dev/null)

  INFERRED=$((TRIPLES_AFTER - TRIPLES_BEFORE))

  echo "  Triples: ${TRIPLES_BEFORE} -> ${TRIPLES_AFTER} (+${INFERRED} inferred)"
  echo "  Classes: ${CLASSES}, Properties: ${PROPERTIES}"
  echo "  Fetch: ${FETCH_MS}ms, Reason: ${REASON_MS}ms"

  # Append JSON
  if [ "$FIRST" = true ]; then
    FIRST=false
  else
    echo ","  >> "$RESULTS_FILE"
  fi
  cat >> "$RESULTS_FILE" <<ENDJSON
  {"id":"${ID}","classes":${CLASSES},"properties":${PROPERTIES},"triples_before":${TRIPLES_BEFORE},"triples_after":${TRIPLES_AFTER},"inferred":${INFERRED},"fetch_ms":${FETCH_MS},"reason_ms":${REASON_MS}}
ENDJSON

done

echo ""  >> "$RESULTS_FILE"
echo "]" >> "$RESULTS_FILE"

echo ""
echo "=== DONE ==="
echo "Results saved to ${RESULTS_FILE}"
echo ""
python3 -c "
import json
with open('${RESULTS_FILE}') as f:
    data = json.load(f)
print(f'{'ID':<20} {'Classes':>8} {'Props':>8} {'Triples':>10} {'After':>10} {'Inferred':>10} {'Fetch':>8} {'Reason':>8}')
print('-' * 92)
for r in data:
    print(f'{r[\"id\"]:<20} {r[\"classes\"]:>8} {r[\"properties\"]:>8} {r[\"triples_before\"]:>10} {r[\"triples_after\"]:>10} {r[\"inferred\"]:>10} {str(r[\"fetch_ms\"])+\"ms\":>8} {str(r[\"reason_ms\"])+\"ms\":>8}')
total_triples = sum(r['triples_before'] for r in data)
total_inferred = sum(r['inferred'] for r in data)
print('-' * 92)
print(f'{'TOTAL':<20} {'':>8} {'':>8} {total_triples:>10} {total_triples+total_inferred:>10} {total_inferred:>10}')
"
