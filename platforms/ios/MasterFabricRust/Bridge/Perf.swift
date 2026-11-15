import Foundation
import QuartzCore

/// App-level, platform-specific performance measurements.
/// Results are sent to Rust via `Action.recordPerf` and rendered from Rust state.
enum Perf {
    /// Process start time from the kernel, seconds since the Unix epoch.
    static var processStart: Double? {
        var kinfo = kinfo_proc()
        var size = MemoryLayout<kinfo_proc>.stride
        var mib: [Int32] = [CTL_KERN, KERN_PROC, KERN_PROC_PID, getpid()]
        guard sysctl(&mib, 4, &kinfo, &size, nil, 0) == 0 else { return nil }
        let tv = kinfo.kp_proc.p_un.__p_starttime
        return Double(tv.tv_sec) + Double(tv.tv_usec) / 1_000_000
    }

    static func nowEpoch() -> Double { Date().timeIntervalSince1970 }
    static func now() -> UInt64 { DispatchTime.now().uptimeNanoseconds }
    static func us(_ start: UInt64) -> Double { Double(now() - start) / 1000 }

    /// Values measured once.
    static var coreInitUs: Double = 0
    static var launchToFirstFrameMs: Double?

    /// Write JSON to Documents/perf.json (automation / before-after comparison).
    static func export(_ dict: [String: Any]) {
        guard let dir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first,
              let data = try? JSONSerialization.data(withJSONObject: dict, options: [.prettyPrinted, .sortedKeys])
        else { return }
        try? data.write(to: dir.appendingPathComponent("perf.json"))
    }
}
