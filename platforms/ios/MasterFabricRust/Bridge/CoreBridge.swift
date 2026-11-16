import Foundation
import Observation
import QuartzCore
import SwiftUI

/// SwiftUI-side mirror of the Rust state.
///
/// No business logic lives here: user events go to Rust via `dispatch`, Rust sends back a
/// *partial* `ViewUpdate` (only the screen state that changed) through `StateListener`,
/// and SwiftUI renders it. Each slice is a separate observable property, so a benchmark
/// progress tick does not invalidate the Home list and vice versa.
@Observable
final class CoreBridge {
    private(set) var revision: UInt64
    private(set) var stack: [NavEntry]
    private(set) var home: HomeState
    private(set) var details: [DetailState]
    private(set) var benchmark: BenchmarkState
    private(set) var info: InfoState
    private(set) var perf: [PerfMetric]
    private(set) var measuringLatency = false
    /// Last details, so a screen being popped keeps its content during the back animation.
    @ObservationIgnored private var detailCache: [String: DetailState] = [:]

    @ObservationIgnored let core: AppCore
    @ObservationIgnored private var listener: Listener?
    @ObservationIgnored private let runner = SwiftRunner()

    init() {
        let t0 = Perf.now()
        let core = AppCore()
        Perf.coreInitUs = Perf.us(t0)
        self.core = core
        let s = core.state()
        revision = s.revision
        stack = s.stack
        home = s.home
        details = s.details
        benchmark = s.benchmark
        info = s.info
        perf = s.perf
        let listener = Listener(bridge: self)
        self.listener = listener
        core.setListener(listener: listener)
        core.setSwiftRunner(runner: runner)
    }

    func dispatch(_ action: Action) {
        core.dispatch(action: action)
    }

    /// Apply only the slices present in the update; untouched slices keep their identity.
    fileprivate func apply(_ u: ViewUpdate) {
        guard u.revision >= revision else { return } // drop out-of-order notifications
        revision = u.revision
        if let v = u.stack { stack = v }
        if let v = u.home { home = v }
        if let v = u.details {
            for d in v { detailCache[d.key] = d }
            details = v
        }
        if let v = u.info { info = v }
        if let v = u.perf { perf = v }
        if let v = u.benchmark {
            let finished = benchmark.running && !v.running
            let ffiFinished = benchmark.ffi.running && !v.ffi.running
            benchmark = v
            if finished {
                exportPerf()
                // `-autorun YES -ffiAutorun YES`: run test #37 after the compute tests, so its
                // break-even uses the measured compute geomean.
                if UserDefaults.standard.bool(forKey: "ffiAutorun"), !ffiAutorunDone {
                    ffiAutorunDone = true
                    DispatchQueue.main.async { self.dispatch(.runFfiBench) }
                }
            }
            if ffiFinished { exportFfi() }
        }
    }

    // MARK: Navigation (the stack lives in Rust)

    /// `NavigationStack(path:)` binding. Pushes only come from Rust; when the user swipes
    /// back or taps Back, Rust receives `popTo`.
    var path: Binding<[String]> {
        Binding(
            get: { self.stack.dropFirst().map(\.key) },
            set: { newValue in
                let current = self.stack.count - 1
                if newValue.count < current {
                    self.dispatch(.popTo(pathLen: UInt32(newValue.count)))
                }
            }
        )
    }

    func detail(for key: String) -> DetailState? {
        details.first { $0.key == key } ?? detailCache[key]
    }

    func metric(_ id: String) -> PerfMetric? { perf.first { $0.id == id } }

    // MARK: Measurement: dispatch → state applied / → next rendered frame

    /// Sends `n` real state changes (alternating the search query) and times
    /// `dispatch` → partial `ViewUpdate` applied to the SwiftUI model. The listener runs
    /// synchronously on the main thread, so when `dispatch` returns the model is updated.
    /// Also times a full `state()` snapshot for comparison with the partial update.
    func measureDispatchLatency(n: Int = 300) -> (avgUs: Double, p95Us: Double, snapshotUs: Double) {
        let q = home.query
        let alt = q + " "
        var samples: [Double] = []
        samples.reserveCapacity(n)
        for i in 0..<n {
            let t0 = Perf.now()
            dispatch(.setQuery(query: i % 2 == 0 ? alt : q))
            samples.append(Perf.us(t0))
        }
        if home.query != q { dispatch(.setQuery(query: q)) }
        samples.sort()
        let avg = samples.reduce(0, +) / Double(n)
        let p95 = samples[min(n - 1, Int(Double(n) * 0.95))]
        let t1 = Perf.now()
        for _ in 0..<n { _ = core.state() }
        let snap = Perf.us(t1) / Double(n)
        return (avg, p95, snap)
    }

    /// dispatch → next display frame (the frame that renders the new state).
    @MainActor
    func measureRenderLatency(samples n: Int = 20) async -> (avgMs: Double, p95Ms: Double) {
        let q = home.query
        var out: [Double] = []
        for i in 0..<n {
            let t0 = CACurrentMediaTime()
            dispatch(.setQuery(query: i % 2 == 0 ? q + " " : q))
            let t1 = await FrameProbe.nextFrame()
            out.append((t1 - t0) * 1000)
        }
        if home.query != q { dispatch(.setQuery(query: q)) }
        out.sort()
        return (out.reduce(0, +) / Double(n), out[min(n - 1, Int(Double(n) * 0.95))])
    }

    /// Runs both probes and records the numbers into Rust (`recordPerf`) so every screen
    /// renders them from Rust state.
    @MainActor
    func measureAndRecordLatency() async {
        guard !measuringLatency else { return }
        measuringLatency = true
        defer { measuringLatency = false }
        let l = measureDispatchLatency()
        lastLatency = l
        record("dispatch_apply_avg_us", "Dispatch → state applied (avg)", l.avgUs, "µs",
               "300 real query changes; Rust reducer + partial ViewUpdate + FFI + Swift apply")
        record("dispatch_apply_p95_us", "Dispatch → state applied (p95)", l.p95Us, "µs", "same samples, 95th percentile")
        record("full_snapshot_us", "Full state snapshot over FFI", l.snapshotUs, "µs",
               "AppCore.state(): whole AppViewState for comparison with partial updates")
        let r = await measureRenderLatency()
        lastRender = r
        record("dispatch_render_ms", "Dispatch → next frame (avg)", r.avgMs, "ms",
               "20 samples, CADisplayLink; includes waiting for the next vsync")
    }

    func recordLaunchMetrics() {
        if let l = Perf.launchToFirstFrameMs {
            record("launch_to_first_frame_ms", "Launch → first frame", l, "ms", "process start (kernel) → root view onAppear")
        }
        record("core_init_us", "Rust core init", Perf.coreInitUs, "µs", "AppCore() constructor incl. seed data")
    }

    private func record(_ id: String, _ label: String, _ value: Double, _ unit: String, _ detail: String) {
        dispatch(.recordPerf(id: id, label: label, value: value, unit: unit, detail: detail))
    }

    @ObservationIgnored var lastLatency: (avgUs: Double, p95Us: Double, snapshotUs: Double)?
    @ObservationIgnored var lastRender: (avgMs: Double, p95Ms: Double)?

    func exportPerf() {
        var rows: [[String: Any]] = []
        for r in benchmark.rows {
            var d: [String: Any] = ["id": r.id, "category": r.category]
            if let m = r.rust { d["rust_ms"] = m.avgMs }
            if let m = r.rustParallel { d["rust_par_ms"] = m.avgMs }
            if let m = r.swift { d["swift_ms"] = m.avgMs }
            if let x = r.ratio { d["ratio"] = x }
            if let x = r.parallelSpeedup { d["parallel_speedup"] = x }
            if let c = r.checksumMatch { d["checksum_match"] = c }
            rows.append(d)
        }
        var out: [String: Any] = ["rows": rows, "core_init_us": Perf.coreInitUs,
                                  "rust_threads": Int(benchmark.rustThreads)]
        if let t = benchmark.lastTotalMs { out["total_ms"] = t }
        if let s = benchmark.summary { out["summary"] = s }
        if let l = Perf.launchToFirstFrameMs { out["launch_to_first_frame_ms"] = l }
        if let l = lastLatency {
            out["dispatch_apply_avg_us"] = l.avgUs
            out["dispatch_apply_p95_us"] = l.p95Us
            out["full_snapshot_us"] = l.snapshotUs
        }
        if let r = lastRender {
            out["dispatch_render_avg_ms"] = r.avgMs
            out["dispatch_render_p95_ms"] = r.p95Ms
        }
        Perf.export(out)
    }

    /// Test #37 results → Documents/ffi.json (automation, see scripts/measure-ffi.sh).
    func exportFfi() {
        let f = benchmark.ffi
        let rows: [[String: Any]] = f.rows.map { r in
            var d: [String: Any] = ["id": r.id, "group": r.group, "case": r.caseId, "calls": r.calls, "reps": r.reps]
            if let s = r.swiftInline { d["swift_inline_ns"] = s.nsPerCall }
            if let s = r.rustPerCall { d["rust_per_call_ns"] = s.nsPerCall }
            if let s = r.rustBatched { d["rust_batched_ns"] = s.nsPerCall }
            if let x = r.overheadNs { d["overhead_ns"] = x }
            if let c = r.checksumMatch { d["checksum_match"] = c }
            return d
        }
        var out: [String: Any] = ["rows": rows, "simulator": DeviceInfo.isSimulator]
        if let c = f.crossingNs { out["crossing_ns"] = c }
        if let s = f.summary { out["summary"] = s }
        if let t = f.lastTotalMs { out["total_ms"] = t }
        Perf.export(out, file: "ffi.json")
    }

    // MARK: Launch arguments for debugging / automation
    //   -startScreen home|detail|benchmark|info   -detailId 2   -autorun YES   -query embassy   -probe YES
    //   -ffiAutorun YES (test #37)   -scrollTo ffi
    @ObservationIgnored private var launchApplied = false
    @ObservationIgnored private var ffiAutorunDone = false

    @MainActor
    func applyLaunchOptions() async {
        guard !launchApplied else { return }
        launchApplied = true
        recordLaunchMetrics()
        let d = UserDefaults.standard
        if let q = d.string(forKey: "query"), !q.isEmpty { dispatch(.setQuery(query: q)) }
        switch d.string(forKey: "startScreen") {
        case "detail":
            let id = d.integer(forKey: "detailId")
            dispatch(.openDetail(itemId: UInt32(max(id, 1))))
        case "benchmark":
            dispatch(.openBenchmark)
        case "info":
            dispatch(.openInfo)
        default:
            break
        }
        if d.bool(forKey: "probe") {
            await measureAndRecordLatency()
        }
        if d.bool(forKey: "autorun") {
            dispatch(.runAllBenchmarks)
        } else if d.bool(forKey: "ffiAutorun") {
            ffiAutorunDone = true
            dispatch(.runFfiBench)
        }
    }
}

/// Awaits the next CADisplayLink tick.
@MainActor
private final class FrameProbe: NSObject {
    private var link: CADisplayLink?
    private var cont: CheckedContinuation<Double, Never>?

    static func nextFrame() async -> Double {
        let p = FrameProbe()
        return await withCheckedContinuation { c in
            p.cont = c
            let link = CADisplayLink(target: p, selector: #selector(tick(_:)))
            p.link = link
            link.add(to: .main, forMode: .common)
        }
    }

    @objc private func tick(_ l: CADisplayLink) {
        l.invalidate()
        link = nil
        cont?.resume(returning: CACurrentMediaTime())
        cont = nil
    }
}

/// Listener Rust calls from any thread; hops the update to the main thread.
private final class Listener: StateListener, @unchecked Sendable {
    weak var bridge: CoreBridge?
    init(bridge: CoreBridge) { self.bridge = bridge }

    func onUpdate(update: ViewUpdate) {
        if Thread.isMainThread {
            bridge?.apply(update)
        } else {
            DispatchQueue.main.async { [weak self] in self?.bridge?.apply(update) }
        }
    }
}
