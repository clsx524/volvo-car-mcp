#!/bin/bash
# Container smoke test for volvo-car-mcp. Usage: smoke-container.sh [image]
set -u
IMAGE=${1:-volvo-car-mcp:latest}
PORT=18901
TOKEN="container-uuid-1111"
BASE="http://127.0.0.1:$PORT"

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

rm -rf /tmp/volvo-mcp-container-state
docker rm -f volvo-mcp-smoke >/dev/null 2>&1
docker run -d --name volvo-mcp-smoke -p "127.0.0.1:$PORT:8811" \
  -e VC_MCP_BEARER_TOKEN="$TOKEN" \
  -e VOLVO_CONFIG_DIR=/data \
  -v /tmp/volvo-mcp-container-state:/data \
  "$IMAGE" >/dev/null || exit 1
trap 'docker rm -f volvo-mcp-smoke >/dev/null 2>&1' EXIT

for _ in $(seq 1 100); do
  curl -sf "$BASE/healthz" >/dev/null 2>&1 && break
  sleep 0.2
done
curl -sf "$BASE/healthz" >/dev/null || { echo "FAIL: server did not come up"; docker logs volvo-mcp-smoke; exit 1; }

echo "== binary version =="
docker run --rm --entrypoint /usr/local/bin/vc-cli "$IMAGE" --version

echo "== healthz (no auth) =="
curl -s -w " [%{http_code}]\n" "$BASE/healthz"

echo "== /mcp no auth =="
curl -s -o /dev/null -w "%{http_code}\n" "$BASE/mcp"

echo "== /mcp wrong token =="
curl -s -o /dev/null -w "%{http_code}\n" -H "Authorization: Bearer nope" "$BASE/mcp"

echo "== initialize =="
INIT_HEADERS=$(mktemp)
curl -s -D "$INIT_HEADERS" -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"smoke","version":"0"}}}' \
  "$BASE/mcp" | rpc | python3 -c "import sys,json; d=json.loads(sys.stdin.read().splitlines()[-1]); r=d['result']; print(r['protocolVersion'], r['serverInfo'])"
SID=$(grep -i '^mcp-session-id' "$INIT_HEADERS" | tr -d '\r' | awk '{print $2}')
echo "session: ${SID:-NONE}"

curl -s -o /dev/null -H "Authorization: Bearer $TOKEN" -H "Mcp-Session-Id: $SID" \
  -H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' "$BASE/mcp"

echo "== tools/list count =="
curl -s -H "Authorization: Bearer $TOKEN" -H "Mcp-Session-Id: $SID" \
  -H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' "$BASE/mcp" \
  | rpc | python3 -c "import sys,json; d=json.loads(sys.stdin.read().splitlines()[-1]); ts=d['result']['tools']; print(len(ts)); assert len(ts)==30, 'expected 30 tools'; assert all('inputSchema' in t for t in ts), 'missing schemas'; print('schemas ok')"

echo "== tools/call auth_whoami (isError expected) =="
curl -s -H "Authorization: Bearer $TOKEN" -H "Mcp-Session-Id: $SID" \
  -H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"auth_whoami","arguments":{}}}' \
  "$BASE/mcp" | rpc | python3 -c "import sys,json; d=json.loads(sys.stdin.read().splitlines()[-1]); r=d['result']; assert r['isError'] is True; print('isError:', r['content'][0]['text'])"

echo "== tools/call vehicle_list (VCC_API_KEY set, no session -> isError) =="
docker rm -f volvo-mcp-smoke >/dev/null 2>&1
docker run -d --name volvo-mcp-smoke -p "127.0.0.1:$PORT:8811" \
  -e VC_MCP_BEARER_TOKEN="$TOKEN" -e VCC_API_KEY=dummy-key -e VOLVO_CONFIG_DIR=/data \
  -v /tmp/volvo-mcp-container-state:/data \
  "$IMAGE" >/dev/null
for _ in $(seq 1 100); do
  curl -sf "$BASE/healthz" >/dev/null 2>&1 && break
  sleep 0.2
done
INIT_HEADERS=$(mktemp)
curl -s -D "$INIT_HEADERS" -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"smoke","version":"0"}}}' \
  "$BASE/mcp" >/dev/null
SID=$(grep -i '^mcp-session-id' "$INIT_HEADERS" | tr -d '\r' | awk '{print $2}')
curl -s -o /dev/null -H "Authorization: Bearer $TOKEN" -H "Mcp-Session-Id: $SID" \
  -H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' "$BASE/mcp"
curl -s -H "Authorization: Bearer $TOKEN" -H "Mcp-Session-Id: $SID" \
  -H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"vehicle_list","arguments":{}}}' \
  "$BASE/mcp" | rpc | python3 -c "import sys,json; d=json.loads(sys.stdin.read().splitlines()[-1]); r=d['result']; assert r['isError'] is True; print('isError:', r['content'][0]['text'][:120])"

echo "ALL CONTAINER CHECKS PASSED"
