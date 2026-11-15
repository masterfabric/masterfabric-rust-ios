#!/usr/bin/env bash
# Rust çekirdeğini iOS (cihaz + simülatör) için RELEASE derler,
# UniFFI ile Swift köprüsünü üretir ve AppCore.xcframework'ü oluşturur.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
# shellcheck disable=SC1091
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

export IPHONEOS_DEPLOYMENT_TARGET=17.0
CRATE=app-ffi
LIB=libapp_ffi
GEN="$ROOT/build/generated"
HEADERS="$ROOT/build/headers"
IOS="$ROOT/platforms/ios"
XCF="$IOS/Frameworks/AppCore.xcframework"

echo "==> Rust: aarch64-apple-ios (cihaz)"
cargo build -p "$CRATE" --release --target aarch64-apple-ios
echo "==> Rust: aarch64-apple-ios-sim (Apple Silicon simülatör)"
cargo build -p "$CRATE" --release --target aarch64-apple-ios-sim
echo "==> Rust: host dylib (UniFFI metadata için)"
cargo build -p "$CRATE" --release

echo "==> UniFFI: Swift bağlamaları"
rm -rf "$GEN" "$HEADERS" && mkdir -p "$GEN" "$HEADERS"
cargo run -q -p uniffi-bindgen -- generate \
  --library "target/release/$LIB.dylib" \
  --language swift \
  --out-dir "$GEN"
mkdir -p "$IOS/MasterFabricRust/Generated"
cp "$GEN"/*.swift "$IOS/MasterFabricRust/Generated/"
cp "$GEN"/*.h "$HEADERS/"
cat "$GEN"/*.modulemap > "$HEADERS/module.modulemap"

echo "==> XCFramework"
rm -rf "$XCF"
xcodebuild -create-xcframework \
  -library "target/aarch64-apple-ios/release/$LIB.a" -headers "$HEADERS" \
  -library "target/aarch64-apple-ios-sim/release/$LIB.a" -headers "$HEADERS" \
  -output "$XCF" >/dev/null

echo "==> Xcode projesi (XcodeGen)"
(cd "$IOS" && xcodegen generate --quiet)

echo "Tamam: $XCF"
