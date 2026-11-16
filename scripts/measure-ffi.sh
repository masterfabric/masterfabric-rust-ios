#!/usr/bin/env bash
# Test #37 (FFI boundary): launches the app N times on the booted simulator, runs only the
# FFI section, collects Documents/ffi.json as build/perf/ffi-<label>-<i>.json and prints
# the per-row median.   scripts/measure-ffi.sh <label> [runs=3]
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
  rm -f "$DATA/Documents/ffi.json"
  sleep 1
  xcrun simctl launch "$SIM_ID" "$BUNDLE_ID" -startScreen benchmark -ffiAutorun YES -scrollTo ffi >/dev/null
  for _ in $(seq 1 240); do [ -f "$DATA/Documents/ffi.json" ] && break; sleep 0.5; done
  sleep 0.5
  cp "$DATA/Documents/ffi.json" "$OUT/ffi-$LABEL-$i.json"
  echo "run $i ok"
done
python3 "$ROOT/scripts/ffi_summary.py" "$OUT"/ffi-"$LABEL"-*.json
