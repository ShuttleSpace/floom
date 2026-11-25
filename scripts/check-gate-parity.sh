#!/usr/bin/env bash
# Every gate in gate.sh must say where it runs, and every mapping must name a
# gate that still exists. Drift in either direction is how "we have a gate" turns
# into "we had a gate".
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mapped="$(grep -vE '^[[:space:]]*(#|$)' "$root/scripts/gate-ci-map.txt" | awk '{print $1}' | sort -u)"
real="$(bash "$root/scripts/gate.sh" --list | awk 'NF {print $1}' | sort -u)"

status=0
for gate in $real; do
  if ! echo "$mapped" | grep -qx "$gate"; then
    echo "FAIL parity: gate '$gate' runs in gate.sh but scripts/gate-ci-map.txt does not say where"
    status=1
  fi
done
for gate in $mapped; do
  if ! echo "$real" | grep -qx "$gate"; then
    echo "FAIL parity: scripts/gate-ci-map.txt maps '$gate', which gate.sh no longer has"
    status=1
  fi
done
if [ "$status" -ne 0 ]; then exit 1; fi
echo "ok: $(echo "$real" | wc -l | tr -d ' ') gates, every one of them says where it runs"
