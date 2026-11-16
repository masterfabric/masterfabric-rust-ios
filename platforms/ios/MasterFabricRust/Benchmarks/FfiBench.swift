import Foundation

/// Test #37: FFI boundary ("chatty calls"). Swift twin of `crates/app-core/src/ffi_bench.rs`.
///
/// The 36 compute tests time each workload inside its own language, so the UniFFI crossing
/// (one per test) is outside the timer. Here the crossing *is* the workload: Swift calls a
/// tiny Rust function once per operation. Timing must happen on the Swift side because only
/// Swift can observe the Swift→Rust call; which cases run, with which parameters, and how
/// the results are interpreted is decided in Rust.
///
/// Every sample: `reps` repetitions of `calls` chained operations, seeded with `0x5EED ^ rep`
/// (so no repetition can be hoisted by the optimizer), XOR-folded into a checksum that must
/// equal Rust's `reference_checksum`.
enum FfiBench {
    static let swiftInline = "swift_inline"
    static let rustPerCall = "rust_per_call"
    static let rustBatched = "rust_batched"

    // MARK: Tiny ops (identical to Rust)

    @inline(__always) static func mix(_ acc: UInt64, _ i: UInt64) -> UInt64 {
        let x = (acc ^ i) &* 0x9E37_79B9_7F4A_7C15
        return (x << 5) | (x >> 59)
    }

    @inline(__always) static func strMix(_ s: String, _ seed: UInt64) -> UInt64 {
        var a = seed
        for b in s.utf8 { a = mix(a, UInt64(b)) }
        return a
    }

    @inline(__always) static func pointMix(_ p: FfiPoint) -> UInt64 {
        mix(mix(mix(0, UInt64(UInt32(bitPattern: p.x))), UInt64(UInt32(bitPattern: p.y))),
            UInt64(UInt32(bitPattern: p.z)))
    }

    @inline(__always) static func vecMix(_ v: [UInt32], _ seed: UInt64) -> UInt64 {
        var a = seed
        for x in v { a = mix(a, UInt64(x)) }
        return a
    }

    @inline(__always) static func point(_ i: UInt64) -> FfiPoint {
        let x = Int32(truncatingIfNeeded: i)
        return FfiPoint(x: x, y: x &* 3, z: 0 &- x)
    }

    /// Same inputs as Rust (fetched from Rust once, outside any timed region).
    static let strings: [String] = ffiSampleStrings()
    static let vec: [UInt32] = ffiSampleVec()

    // MARK: One sample

    @inline(never)
    static func sample(_ caseId: String, _ variant: String, calls: UInt64, reps: UInt64) -> UInt64? {
        var total: UInt64 = 0
        let strings = Self.strings
        let vec = Self.vec
        switch (caseId, variant) {
        case ("u64", swiftInline):
            for r in 0..<reps {
                var a: UInt64 = 0x5EED ^ r
                for i in 0..<calls { a = mix(a, i) }
                total ^= a
            }
        case ("u64", rustPerCall):
            for r in 0..<reps {
                var a: UInt64 = 0x5EED ^ r
                for i in 0..<calls { a = ffiMix(acc: a, i: i) }
                total ^= a
            }
        case ("u64", rustBatched):
            for r in 0..<reps {
                total ^= ffiMixBatch(seed: 0x5EED ^ r, n: calls)
            }
        case ("string", swiftInline):
            for r in 0..<reps {
                var a: UInt64 = 0x5EED ^ r
                for i in 0..<calls { a = mix(a, strMix(strings[Int(i & 15)], i)) }
                total ^= a
            }
        case ("string", rustPerCall):
            for r in 0..<reps {
                var a: UInt64 = 0x5EED ^ r
                for i in 0..<calls { a = mix(a, ffiStrMix(s: strings[Int(i & 15)], seed: i)) }
                total ^= a
            }
        case ("record", swiftInline):
            for r in 0..<reps {
                var a: UInt64 = 0x5EED ^ r
                for i in 0..<calls { a = mix(a, pointMix(point(i))) }
                total ^= a
            }
        case ("record", rustPerCall):
            for r in 0..<reps {
                var a: UInt64 = 0x5EED ^ r
                for i in 0..<calls { a = mix(a, ffiPointMix(p: point(i))) }
                total ^= a
            }
        case ("vec16", swiftInline):
            for r in 0..<reps {
                var a: UInt64 = 0x5EED ^ r
                for i in 0..<calls { a = mix(a, vecMix(vec, i)) }
                total ^= a
            }
        case ("vec16", rustPerCall):
            for r in 0..<reps {
                var a: UInt64 = 0x5EED ^ r
                for i in 0..<calls { a = mix(a, ffiVecMix(v: vec, seed: i)) }
                total ^= a
            }
        default:
            return nil
        }
        return total
    }

    /// 1 untimed warm-up sample + `samples` timed ones; median ns per operation.
    static func measure(caseId: String, variant: String, calls: UInt64, reps: UInt64, samples: UInt32) -> FfiSample? {
        guard let checksum = sample(caseId, variant, calls: calls, reps: reps) else { return nil }
        var times: [Double] = []
        for _ in 0..<max(samples, 1) {
            let t0 = DispatchTime.now().uptimeNanoseconds
            guard let c = sample(caseId, variant, calls: calls, reps: reps) else { return nil }
            let ns = Double(DispatchTime.now().uptimeNanoseconds - t0)
            blackHole(c)
            guard c == checksum else { return FfiSample(nsPerCall: ns, checksum: c) }
            times.append(ns / Double(calls * reps))
        }
        times.sort()
        return FfiSample(nsPerCall: times[times.count / 2], checksum: checksum)
    }
}
