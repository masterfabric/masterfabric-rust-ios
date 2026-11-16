#!/usr/bin/env bash
# Host cross-check: runs every benchmark once in Rust (release) and in Swift (swiftc -O),
# compares checksums and prints single-run timings. No simulator needed.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"
OUT="$ROOT/build/crosscheck"; mkdir -p "$OUT"
cargo run -q --release -p app-core --example calibrate > "$OUT/rust.txt"
B=platforms/ios/MasterFabricRust/Benchmarks
cat > "$OUT/main.swift" <<'SWIFT'
import Foundation
struct BenchMeasurement { var avgMs: Double; var minMs: Double; var iterations: UInt32; var checksum: UInt64 }
let a = CommandLine.arguments
var i = 1
while i + 1 < a.count {
    let t0 = DispatchTime.now().uptimeNanoseconds
    let c = SwiftWorkloads.runOnce(a[i], UInt64(a[i + 1])!)!
    print(a[i], Double(DispatchTime.now().uptimeNanoseconds - t0) / 1e6, String(c, radix: 16))
    i += 2
}
SWIFT
xcrun swiftc -O -swift-version 5 -o "$OUT/swiftbench" "$OUT/main.swift" $B/SwiftWorkloads.swift $B/SwiftMedia.swift $B/SwiftHeavy.swift
# shellcheck disable=SC2046
"$OUT/swiftbench" $(awk 'NF>5 && $1!="est." {print $1, $3}' "$OUT/rust.txt") > "$OUT/swift.txt"
python3 - "$OUT/rust.txt" "$OUT/swift.txt" <<'PY'
import math, re, sys
rust = {}
for l in open(sys.argv[1]):
    m = re.match(r'(\S+)\s+(\S+)\s+(\d+)\s+([\d.]+) ms.*chk (\w+)', l)
    if m: rust[m[1]] = (m[2], float(m[4]), int(m[5], 16))
swift = {}
for l in open(sys.argv[2]):
    k, ms, c = l.split()
    swift[k] = (float(ms), int(c, 16))
bad, logs = 0, []
print(f"{'test':12s} {'category':11s} {'rust ms':>8s} {'swift ms':>9s} {'ratio':>6s}  checksum")
for k, (cat, ms, c) in rust.items():
    sm, sc = swift.get(k, (float('nan'), None))
    ok = sc == c
    bad += not ok
    r = sm / ms if ms else float('nan')
    logs.append(math.log(r))
    print(f"{k:12s} {cat:11s} {ms:8.2f} {sm:9.2f} {r:6.2f}  {'ok' if ok else 'MISMATCH'}")
print(f"\n{len(rust)} tests, geomean Swift/Rust = {math.exp(sum(logs)/len(logs)):.2f}, mismatches: {bad}")
sys.exit(1 if bad else 0)
PY
