#!/usr/bin/env bash
# Builds the Rust core in RELEASE for iOS (device + simulator),
# generates the Swift bridge with UniFFI and creates AppCore.xcframework.
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

echo "==> Rust: aarch64-apple-ios (device)"
cargo build -p "$CRATE" --release --target aarch64-apple-ios
echo "==> Rust: aarch64-apple-ios-sim (Apple Silicon simulator)"
cargo build -p "$CRATE" --release --target aarch64-apple-ios-sim
echo "==> Rust: host dylib (for UniFFI metadata)"
cargo build -p "$CRATE" --release

echo "==> UniFFI: Swift bindings"
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

echo "==> Xcode project (XcodeGen)"
(cd "$IOS" && xcodegen generate --quiet)

echo "Done: $XCF"
