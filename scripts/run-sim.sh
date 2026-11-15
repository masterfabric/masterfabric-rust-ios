#!/usr/bin/env bash
# Uygulamayı simülatörde derler, kurar ve açar.
# Açık bir simülatör varsa onu kullanır; yoksa SIM_NAME (varsayılan "iPhone Duo") açılır.
# Ek argümanlar uygulamaya iletilir, ör.:  scripts/run-sim.sh -startScreen benchmark -autorun YES
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IOS="$ROOT/platforms/ios"
CONFIG="${CONFIG:-Release}"
BUNDLE_ID=co.masterfabric.rustios

SIM_ID="$(xcrun simctl list devices booted | grep -Eo '\(([0-9A-F-]{36})\)' | head -1 | tr -d '()' || true)"
if [ -z "$SIM_ID" ]; then
  NAME="${SIM_NAME:-iPhone Duo}"
  SIM_ID="$(xcrun simctl list devices available | grep -F "$NAME (" | grep -Eo '[0-9A-F-]{36}' | head -1)"
  echo "==> Simülatör açılıyor: $NAME ($SIM_ID)"
  xcrun simctl boot "$SIM_ID"
fi
echo "==> Simülatör: $SIM_ID"

xcodebuild -project "$IOS/MasterFabricRust.xcodeproj" -scheme MasterFabricRust \
  -configuration "$CONFIG" -destination "id=$SIM_ID" \
  -derivedDataPath "$ROOT/build/DerivedData" build -quiet
APP="$ROOT/build/DerivedData/Build/Products/$CONFIG-iphonesimulator/MasterFabricRust.app"
xcrun simctl install "$SIM_ID" "$APP"
xcrun simctl terminate "$SIM_ID" "$BUNDLE_ID" 2>/dev/null || true
xcrun simctl launch "$SIM_ID" "$BUNDLE_ID" "$@"
