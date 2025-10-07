#!/usr/bin/env bash

set -Eeuo pipefail

# Starts test etcd, runs local maroon instances, and blocks until Ctrl+C.
# On Ctrl+C (or TERM), kills the processes and shuts down etcd compose.

PORT1=${PORT1:-3000}
PORT2=${PORT2:-3001}

pid1=""
pid2=""
pgid1=""
pgid2=""
cleaned=0

cleanup() {
  # Prevent double-execution
  if [[ "$cleaned" -eq 1 ]]; then
    return
  fi
  cleaned=1

  echo "\n[cleanup] Caught signal. Stopping maroon instances and etcd..." >&2

  # Try graceful stop for both process groups if available
  if [[ -n "$pgid1" ]]; then
    echo "[cleanup] Killing group $pgid1 (PORT=$PORT1)" >&2
    kill -TERM -"$pgid1" 2>/dev/null || true
  fi
  if [[ -n "$pgid2" ]]; then
    echo "[cleanup] Killing group $pgid2 (PORT=$PORT2)" >&2
    kill -TERM -"$pgid2" 2>/dev/null || true
  fi

  # Give some time for graceful shutdown
  sleep 2 || true

  # Force kill if still alive
  if [[ -n "$pgid1" ]] && ps -o pgid= -p "$pid1" >/dev/null 2>&1; then
    kill -KILL -"$pgid1" 2>/dev/null || true
  fi
  if [[ -n "$pgid2" ]] && ps -o pgid= -p "$pid2" >/dev/null 2>&1; then
    kill -KILL -"$pgid2" 2>/dev/null || true
  fi

  # Shut down etcd compose stack
  echo "[cleanup] make shutdown-test-etcd" >&2
  make shutdown-test-etcd || true

  echo "[cleanup] Done." >&2
}

trap cleanup INT TERM

echo "[stack] Starting test etcd via 'make start-test-etcd'..."
make start-test-etcd

echo "[stack] Launching maroon instances..."

# Start first instance in background and capture its PID/PGID
(
  PORT=$PORT1 make run-local
) &
pid1=$!
pgid1=$(ps -o pgid= "$pid1" | tr -d ' ')
echo "[stack] Started PORT=$PORT1 with pid=$pid1 pgid=$pgid1"

# Start second instance in background and capture its PID/PGID
(
  PORT=$PORT2 make run-local
) &
pid2=$!
pgid2=$(ps -o pgid= "$pid2" | tr -d ' ')
echo "[stack] Started PORT=$PORT2 with pid=$pid2 pgid=$pgid2"

echo "[stack] Press Ctrl+C to stop both instances and shutdown etcd."

# Wait for background jobs; if any exits, continue waiting for the other until Ctrl+C
wait -n $pid1 $pid2 || true
wait $pid1 $pid2 || true

# If we reach here without a signal, still cleanup to shutdown etcd
cleanup

