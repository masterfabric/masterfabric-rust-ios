import Foundation

/// Object the Rust core calls to measure the same workload in Swift.
/// Rust calls it from its own background thread; the main thread is never blocked.
final class SwiftRunner: SwiftBenchRunner, @unchecked Sendable {
    func language() -> String { "Swift" }

    func run(benchId: String, param: UInt64, iterations: UInt32) -> BenchMeasurement? {
        assert(!Thread.isMainThread, "benchmark must not run on the main thread")
        return SwiftWorkloads.measure(benchId, param: param, iterations: iterations)
    }
}
