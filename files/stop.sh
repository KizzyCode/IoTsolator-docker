#!/bin/sh
set -euo pipefail

# Register signal handler for SIGINT
function onstop() {
    echo "shutting down $INTERFACE"
    ifconfig "$INTERFACE" "0.0.0.0"
    ifconfig "$INTERFACE" down
    exit 0
}
trap onstop 2

# Spin until we get a signal
while true; do
    sleep 0.3
done
