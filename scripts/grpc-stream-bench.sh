#!/usr/bin/env bash
# Standalone gRPC streaming benchmark — NOT integrated with benchmark.sh.
# Tries ghz against all four RPC shapes on a single framework container:
#   1. Unary            — GetSum
#   2. Server streaming — StreamSum  (1 request, N replies per call)
#   3. Client streaming — CollectSum (N requests, 1 reply per call)
#   4. Bidirectional    — EchoSum    (N requests, N replies per call)
#
# Usage:  ./scripts/grpc-stream-bench.sh [framework-dir]
# Default framework: aspnet-grpc
#
# Requirements:
#   - docker
#   - ghz (https://ghz.sh/ — `go install github.com/bojand/ghz/cmd/ghz@latest`)
#   - framework's Protos/benchmark.proto must contain: GetSum, StreamSum, CollectSum, EchoSum
#
# The container is started on host networking at port 8080 (h2c) and shut down
# when the script exits.

set -euo pipefail

FRAMEWORK="${1:-aspnet-grpc}"
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
FW_DIR="$ROOT_DIR/frameworks/$FRAMEWORK"
PROTO_FILE=$(find "$FW_DIR" -name "benchmark.proto" -type f | head -1)
IMAGE="httparena-grpc-stream-$FRAMEWORK:local"
CONTAINER="grpc-stream-bench"
PORT=8080
HOST="localhost:$PORT"

# ghz knobs — tune these per workload
UNARY_N=200000           # total unary calls
UNARY_C=128              # concurrent streams
STREAM_CALLS=5000        # number of streaming RPC calls
STREAM_C=128             # concurrent streams
STREAM_MSGS_PER_CALL=100 # messages per call for server/client/bidi streaming
DURATION_CAP=15s         # wall-clock cap per scenario

if [ -z "$PROTO_FILE" ] || [ ! -f "$PROTO_FILE" ]; then
    echo "FAIL: benchmark.proto not found under $FW_DIR"
    exit 1
fi
if ! command -v ghz >/dev/null 2>&1; then
    echo "FAIL: ghz not installed — run: go install github.com/bojand/ghz/cmd/ghz@latest"
    exit 1
fi

cleanup() { docker rm -f "$CONTAINER" >/dev/null 2>&1 || true; }
trap cleanup EXIT

echo "=== Building image $IMAGE ==="
docker build -t "$IMAGE" "$FW_DIR"

echo ""
echo "=== Starting $FRAMEWORK on :$PORT ==="
docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
docker run -d --name "$CONTAINER" --network host \
    --security-opt seccomp=unconfined \
    --ulimit memlock=-1:-1 --ulimit nofile=1048576:1048576 \
    "$IMAGE" >/dev/null

echo "[wait] Waiting for server..."
for i in $(seq 1 30); do
    if ghz --insecure --proto "$PROTO_FILE" \
         --call benchmark.BenchmarkService/GetSum \
         -d '{"a":1,"b":1}' -n 1 -c 1 -t 2s "$HOST" >/dev/null 2>&1; then
        break
    fi
    [ "$i" -eq 30 ] && { echo "FAIL: server did not come up"; exit 1; }
    sleep 1
done
echo "[ready] server up"
echo ""

# Helper — run ghz and print the interesting stats
run_bench() {
    local label="$1"; shift
    echo "=== $label ==="
    ghz --insecure --proto "$PROTO_FILE" \
        -t "$DURATION_CAP" \
        "$@" "$HOST" 2>&1 \
      | grep -E '^\s*(Count|Total|Slowest|Fastest|Average|Requests/sec|Response time histogram|\[|Status code distribution|OK)' \
      || true
    echo ""
}

# 1. Unary — baseline
run_bench "1. Unary (GetSum)  n=$UNARY_N c=$UNARY_C" \
    --call benchmark.BenchmarkService/GetSum \
    -d '{"a":1,"b":2}' \
    -n "$UNARY_N" -c "$UNARY_C"

# 2. Server streaming — one call yields STREAM_MSGS_PER_CALL replies
run_bench "2. Server streaming (StreamSum)  calls=$STREAM_CALLS msgs/call=$STREAM_MSGS_PER_CALL c=$STREAM_C" \
    --call benchmark.BenchmarkService/StreamSum \
    -d "{\"a\":1,\"b\":2,\"count\":$STREAM_MSGS_PER_CALL}" \
    -n "$STREAM_CALLS" -c "$STREAM_C"

# 3. Client streaming — ghz uses -d as an array; each element is one outgoing message
#    Build a JSON array of STREAM_MSGS_PER_CALL {a,b} entries on the fly.
CLIENT_DATA=$(python3 -c "import json; print(json.dumps([{'a':1,'b':2} for _ in range($STREAM_MSGS_PER_CALL)]))")
run_bench "3. Client streaming (CollectSum)  calls=$STREAM_CALLS msgs/call=$STREAM_MSGS_PER_CALL c=$STREAM_C" \
    --call benchmark.BenchmarkService/CollectSum \
    -d "$CLIENT_DATA" \
    -n "$STREAM_CALLS" -c "$STREAM_C"

# 4. Bidirectional — ghz sends each -d array element and reads one reply before sending the next
run_bench "4. Bidirectional (EchoSum)  calls=$STREAM_CALLS msgs/call=$STREAM_MSGS_PER_CALL c=$STREAM_C" \
    --call benchmark.BenchmarkService/EchoSum \
    -d "$CLIENT_DATA" \
    -n "$STREAM_CALLS" -c "$STREAM_C"

echo "=== Done ==="
