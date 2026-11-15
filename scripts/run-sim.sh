#!/usr/bin/env bash
# Builds, installs and launches the app on the simulator.
# Reuses a booted simulator; otherwise boots SIM_NAME (default "iPhone 18 Pro").
# Extra arguments are passed to the app, e.g.:  scripts/run-sim.sh -startScreen benchmark -autorun YES
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IOS="$ROOT/platforms/ios"
CONFIG="${CONFIG:-Release}"
BUNDLE_ID=co.masterfabric.rustios

SIM_ID="$(xcrun simctl list devices booted | grep -Eo '\(([0-9A-F-]{36})\)' | head -1 | tr -d '()' || true)"
if [ -z "$SIM_ID" ]; then
  NAME="${SIM_NAME:-iPhone 18 Pro}"
  SIM_ID="$(xcrun simctl list devices available | grep -F "$NAME (" | grep -Eo '[0-9A-F-]{36}' | head -1)"
  echo "==> Booting simulator: $NAME ($SIM_ID)"
  xcrun simctl boot "$SIM_ID"
fi
echo "==> Simulator: $SIM_ID"

xcodebuild -project "$IOS/MasterFabricRust.xcodeproj" -scheme MasterFabricRust \
  -configuration "$CONFIG" -destination "id=$SIM_ID" \
  -derivedDataPath "$ROOT/build/DerivedData" build -quiet
APP="$ROOT/build/DerivedData/Build/Products/$CONFIG-iphonesimulator/MasterFabricRust.app"
xcrun simctl install "$SIM_ID" "$APP"
xcrun simctl terminate "$SIM_ID" "$BUNDLE_ID" 2>/dev/null || true
xcrun simctl launch "$SIM_ID" "$BUNDLE_ID" "$@"
