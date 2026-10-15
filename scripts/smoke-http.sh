#!/bin/bash
# Local smoke test for the volvo-car-mcp HTTP surface. Run from repo root.
set -u
cd "$(dirname "$0")/.."
PORT=${1:-18811}
TOKEN="test-uuid-0000"

# Prints the JSON-RPC message from a response that may be application/json or SSE.
rpc() { python3 -c '
import sys, json
raw = sys.stdin.read()
if not raw.strip():
    sys.exit("empty response")
if raw.lstrip().startswith("{"):
    print(json.dumps(json.loads(raw)))
else:
    for line in raw.splitlines():
        if line.startswith("data:"):
            payload = line[5:].strip()
            if payload:
                print(json.dumps(json.loads(payload)))
'; }
BASE="http://127.0.0.1:$PORT"

rm -rf /tmp/volvo-mcp-smoke
VC_MCP_BEARER_TOKEN="$TOKEN" VOLVO_CONFIG_DIR=/tmp/volvo-mcp-smoke \
  ./target/release/vc-cli mcp --host 127.0.0.1 --port "$PORT" &
SERVER_PID=$!
trap 'kill $SERVER_PID 2>/dev/null' EXIT
for _ in $(seq 1 50); do
  curl -sf "$BASE/healthz" >/dev/null 2>&1 && break
  sleep 0.2
done

echo "== healthz (no auth) =="
curl -s -w " [%{http_code}]\n" "$BASE/healthz"

echo "== /mcp no auth =="
curl -s -o /dev/null -w "%{http_code}\n" "$BASE/mcp"

echo "== /mcp wrong token =="
curl -s -o /dev/null -w "%{http_code}\n" -H "Authorization: Bearer wrong" "$BASE/mcp"

echo "== initialize =="
INIT_HEADERS=$(mktemp)
curl -s -D "$INIT_HEADERS" -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"smoke","version":"0"}}}' \
  "$BASE/mcp" | head -c 400; echo
SID=$(grep -i '^mcp-session-id' "$INIT_HEADERS" | tr -d '\r' | awk '{print $2}')
echo "session: ${SID:-NONE}"

curl -s -o /dev/null -H "Authorization: Bearer $TOKEN" -H "Mcp-Session-Id: $SID" \
  -H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' "$BASE/mcp"

echo "== tools/list count =="
curl -s -H "Authorization: Bearer $TOKEN" -H "Mcp-Session-Id: $SID" \
  -H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' "$BASE/mcp" \
  | rpc | python3 -c "import sys,json; d=json.loads(sys.stdin.read().splitlines()[-1]); ts=d['result']['tools']; print(len(ts)); print(sorted(t['name'] for t in ts))"

echo "== tools/call auth_whoami (no session -> isError) =="
curl -s -H "Authorization: Bearer $TOKEN" -H "Mcp-Session-Id: $SID" \
  -H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"auth_whoami","arguments":{}}}' \
  "$BASE/mcp" | head -c 500; echo

echo "== tools/call set_default_vin (invalid vin -> isError) =="
curl -s -H "Authorization: Bearer $TOKEN" -H "Mcp-Session-Id: $SID" \
  -H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"set_default_vin","arguments":{"vin":"BOGUS"}}}' \
  "$BASE/mcp" | head -c 500; echo

echo "== tools/call vehicle_list (no auth session -> isError) =="
curl -s -H "Authorization: Bearer $TOKEN" -H "Mcp-Session-Id: $SID" \
  -H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"vehicle_list","arguments":{}}}' \
  "$BASE/mcp" | head -c 500; echo

echo "== bad schema (engine_start without runtime_minutes) =="
curl -s -o /dev/null -w "%{http_code}\n" -H "Authorization: Bearer $TOKEN" -H "Mcp-Session-Id: $SID" \
  -H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"engine_start","arguments":{}}}' \
  "$BASE/mcp"
