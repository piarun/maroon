#!/usr/bin/env bash
# Simple multi-threaded HTTP load script for the gateway.
# - Spawns N background threads that POST to /new_request in a loop
# - Cleanly stops all threads on Ctrl+C
#
# Usage:
#   THREADS=8 GATEWAY_URL=http://localhost:5000 SLEEP_MS=200 ./tests/http_threads.sh
#
# Config (env vars):
#   THREADS     - number of background threads (default: 4)
#   GATEWAY_URL - base URL (default: http://localhost:5000)
#   SLEEP_MS    - delay between requests in each thread (default: 100)
#   PAYLOAD     - JSON payload (default: global.add with two U64 args)

set -u

THREADS=${THREADS:-4}
GATEWAY_URL=${GATEWAY_URL:-http://localhost:5000}
SLEEP_MS=${SLEEP_MS:-100}
PAYLOAD=${PAYLOAD:-'{"fiber_type":"global","function_key":"add","init_values":[{"U64":3003},{"U64":150}]}' }

REQ_URL="$GATEWAY_URL/new_request"

ms_sleep() {
  # Sleep for N milliseconds (supports fractional seconds)
  local ms=$1
  python3 - "$ms" << 'PY' 2>/dev/null || awk -v ms="$ms" 'BEGIN { system("sleep " ms/1000.0) }'
import sys, time
ms = float(sys.argv[1])
time.sleep(ms/1000.0)
PY
}

worker() {
  local id=$1
  while true; do
    ts=$(date -Is)
    code=$(curl -sS -o /dev/null -w "%{http_code}" -X POST "$REQ_URL" \
      -H "Content-Type: application/json" \
      --data "$PAYLOAD" || echo "curl_error")
    echo "[$ts] thread=$id status=$code"
    ms_sleep "$SLEEP_MS"
  done
}

echo "Starting $THREADS threads -> POST $REQ_URL"
echo "Payload: $PAYLOAD"

# Trap Ctrl+C and terminate all background jobs
stop_all() {
  echo "Stopping all threads..."
  jobs -p | xargs -r kill 2>/dev/null || true
  wait
}
trap stop_all INT TERM

# Spawn workers
for i in $(seq 1 "$THREADS"); do
  worker "$i" &
done

# Wait indefinitely until trap fires
wait

