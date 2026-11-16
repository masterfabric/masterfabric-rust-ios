#!/usr/bin/env bash
# Measurement: launches the app N times on the booted simulator (latency probe + all benchmarks)
# and collects each run's Documents/perf.json as build/perf/<label>-<i>.json.
#   scripts/measure.sh <label> [runs=3]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LABEL="${1:?label required}"; RUNS="${2:-3}"
BUNDLE_ID=com.masterfabric.rustios
SIM_ID="$(xcrun simctl list devices booted | grep -Eo '\(([0-9A-F-]{36})\)' | head -1 | tr -d '()')"
[ -n "$SIM_ID" ] || { echo "No booted simulator"; exit 1; }
APP="$ROOT/build/DerivedData/Build/Products/Release-iphonesimulator/MasterFabricRust.app"
xcrun simctl install "$SIM_ID" "$APP"
DATA="$(xcrun simctl get_app_container "$SIM_ID" "$BUNDLE_ID" data)"
OUT="$ROOT/build/perf"; mkdir -p "$OUT"
for i in $(seq 1 "$RUNS"); do
  xcrun simctl terminate "$SIM_ID" "$BUNDLE_ID" 2>/dev/null || true
  while pgrep -f "MasterFabricRust.app/MasterFabricRust" >/dev/null; do sleep 0.2; done
  rm -f "$DATA/Documents/perf.json"
  sleep 1
  xcrun simctl launch "$SIM_ID" "$BUNDLE_ID" -startScreen benchmark -probe YES -autorun YES >/dev/null
  for _ in $(seq 1 120); do [ -f "$DATA/Documents/perf.json" ] && break; sleep 0.5; done
  sleep 0.5
  cp "$DATA/Documents/perf.json" "$OUT/$LABEL-$i.json"
  echo "run $i ok"
done
python3 "$ROOT/scripts/perf_summary.py" "$OUT"/"$LABEL"-*.json
