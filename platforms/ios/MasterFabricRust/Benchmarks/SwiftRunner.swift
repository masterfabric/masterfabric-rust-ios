import Foundation

/// Object the Rust core calls to measure the same workload in Swift.
/// Rust calls it from its own background thread; the main thread is never blocked.
final class SwiftRunner: SwiftBenchRunner, @unchecked Sendable {
    func language() -> String { "Swift" }

    func run(benchId: String, param: UInt64, iterations: UInt32) -> BenchMeasurement? {
        assert(!Thread.isMainThread, "benchmark must not run on the main thread")
        return SwiftWorkloads.measure(benchId, param: param, iterations: iterations)
    }

    /// Test #37: Rust asks Swift to time one FFI-boundary variant (Swift is the caller,
    /// so the Swift→Rust crossing can only be observed here).
    func runFfi(caseId: String, variant: String, calls: UInt64, reps: UInt64, samples: UInt32) -> FfiSample? {
        assert(!Thread.isMainThread, "benchmark must not run on the main thread")
        return FfiBench.measure(caseId: caseId, variant: variant, calls: calls, reps: reps, samples: samples)
    }
}
