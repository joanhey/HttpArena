#!/bin/bash
set -e

/usr/local/bin/swerver --config /etc/swerver/config-h1.json &
H1_PID=$!

/usr/local/bin/swerver --config /etc/swerver/config-tls.json &
TLS_PID=$!

shutdown() {
    kill "$H1_PID" "$TLS_PID" 2>/dev/null || true
    wait "$H1_PID" 2>/dev/null || true
    wait "$TLS_PID" 2>/dev/null || true
    exit 0
}
trap shutdown TERM INT

wait -n "$H1_PID" "$TLS_PID"
shutdown
exit 1
