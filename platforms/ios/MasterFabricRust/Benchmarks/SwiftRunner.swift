import Foundation

/// Rust çekirdeğinin, aynı testi Swift ile ölçmek için çağırdığı nesne.
/// Rust bunu kendi arka plan thread'inden çağırır; ana thread hiç bloklanmaz.
final class SwiftRunner: SwiftBenchRunner, @unchecked Sendable {
    func language() -> String { "Swift" }

    func run(benchId: String, param: UInt64, iterations: UInt32) -> BenchMeasurement? {
        assert(!Thread.isMainThread, "benchmark ana thread'de çalışmamalı")
        return SwiftWorkloads.measure(benchId, param: param, iterations: iterations)
    }
}
