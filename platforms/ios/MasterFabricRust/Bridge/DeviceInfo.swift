import Foundation
import UIKit

/// Benchmark ekranında gösterilen cihaz / derleme bilgisi.
enum DeviceInfo {
    static var isSimulator: Bool {
        #if targetEnvironment(simulator)
        return true
        #else
        return false
        #endif
    }

    static var swiftBuild: String {
        #if DEBUG
        return "Debug"
        #else
        return "Release"
        #endif
    }

    static var machine: String {
        if let model = ProcessInfo.processInfo.environment["SIMULATOR_MODEL_IDENTIFIER"] {
            return model
        }
        var info = utsname()
        uname(&info)
        return withUnsafeBytes(of: &info.machine) { raw in
            String(decoding: raw.prefix { $0 != 0 }, as: UTF8.self)
        }
    }

    static var deviceName: String {
        ProcessInfo.processInfo.environment["SIMULATOR_DEVICE_NAME"] ?? UIDevice.current.model
    }

    static var os: String { "\(UIDevice.current.systemName) \(UIDevice.current.systemVersion)" }
    static var cores: Int { ProcessInfo.processInfo.activeProcessorCount }
    static var memoryGB: String {
        String(format: "%.0f GB", Double(ProcessInfo.processInfo.physicalMemory) / 1_073_741_824)
    }
}
