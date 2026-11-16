import Charts
import SwiftUI

/// Test #37 section: FFI boundary / chatty calls. Its own section, separate from the
/// 8 compute categories. Rows, verdicts and the crossover analysis come from Rust.
struct FfiSectionView: View {
    @Environment(CoreBridge.self) private var bridge
    let ffi: FfiSection
    let busy: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline) {
                Label(ffi.title, systemImage: "arrow.left.arrow.right.circle.fill")
                    .font(.title3.weight(.bold))
                    .foregroundStyle(.teal)
                Spacer()
            }
            .padding(.top, 6)
            Text(ffi.intro).font(.caption).foregroundStyle(.secondary)

            Button {
                bridge.dispatch(.runFfiBench)
            } label: {
                Label(ffi.running ? "Measuring…" : "Run #37 (\(ffi.rows.count) cases)",
                      systemImage: ffi.running ? "hourglass" : "play.fill")
                    .font(.headline)
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .tint(.teal)
            .controlSize(.large)
            .disabled(busy)
            .accessibilityIdentifier("run-ffi")

            if ffi.running || ffi.progress > 0 {
                ProgressView(value: ffi.progress) {
                    HStack {
                        Text(ffi.currentLabel ?? (ffi.running ? "Preparing…" : "Done"))
                        Spacer()
                        Text("\(Int(ffi.progress * 100))%").monospacedDigit()
                    }
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
                .tint(.teal)
            }

            if let c = ffi.crossingNs {
                Card {
                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            Text("Extra cost per Swift→Rust call").font(.caption).foregroundStyle(.secondary)
                            Spacer()
                            if let t = ffi.lastTotalMs {
                                Text(String(format: "%.1f s total", t / 1000))
                                    .font(.caption.monospacedDigit()).foregroundStyle(.secondary)
                            }
                        }
                        HStack(spacing: 8) {
                            cost("u64", c)
                            ForEach(ffi.rows.filter { $0.group == "marshal" }, id: \.id) { r in
                                if let o = r.overheadNs { cost(shortCase(r.caseId), o) }
                            }
                        }
                    }
                }
            }

            Card { table(ffi.rows) }.id("ffi-table")
            Card { chart(ffi.rows.filter { $0.group == "sweep" }) }.id("ffi-chart")

            if let s = ffi.summary {
                Card {
                    Label {
                        Text(s).font(.subheadline)
                    } icon: {
                        Image(systemName: "scope").foregroundStyle(.teal)
                    }
                }
            }
            Text("ns per operation, median of 5 samples (1 warm-up). Each sample ≈ 1,000,000 operations: reps × calls. Swift inline = no FFI; Rust per call = one UniFFI call per op; Rust batched = one UniFFI call doing all ops in Rust. Overhead = per call − inline. At 1–10 calls the inline loop is throughput-bound (independent reps overlap on the CPU), so its ns/op is lower than in long dependent chains. Checksums of all variants must equal Rust's reference."
                 + (DeviceInfo.isSimulator ? " Simulator: Mac CPU, not a real iPhone." : ""))
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .id("ffi")
    }

    private func cost(_ title: String, _ ns: Double) -> some View {
        VStack(spacing: 2) {
            Text("+" + fmt(ns)).font(.headline.weight(.bold).monospacedDigit())
                .foregroundStyle(ns < 5 ? Color.teal : ns < 100 ? Color.orange : Color.red)
            Text(title + " · ns").font(.caption2).foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 6)
        .background(Color(.secondarySystemBackground), in: RoundedRectangle(cornerRadius: 10))
    }

    private func shortCase(_ id: String) -> String {
        switch id {
        case "string": return "String"
        case "record": return "record"
        case "vec16": return "Vec×16"
        default: return id
        }
    }

    // MARK: Table

    @ViewBuilder
    private func table(_ rows: [FfiRow]) -> some View {
        Grid(alignment: .trailing, horizontalSpacing: 8, verticalSpacing: 6) {
            GridRow {
                Text("Calls").gridColumnAlignment(.leading)
                Text("Swift\ninline")
                Text("Rust\nper call")
                Text("Rust\nbatched")
                Text("Over-\nhead")
                Text("")
            }
            .font(.caption2.weight(.semibold))
            .foregroundStyle(.secondary)
            .multilineTextAlignment(.trailing)
            Divider()
            ForEach(rows.filter { $0.group == "sweep" }, id: \.id) { r in row(r) }
            GridRow {
                Text("Marshaling · 10,000 calls")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.teal)
                    .gridCellColumns(6)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.top, 4)
            }
            ForEach(rows.filter { $0.group == "marshal" }, id: \.id) { r in row(r) }
        }
        .font(.caption.monospacedDigit())
    }

    @ViewBuilder
    private func row(_ r: FfiRow) -> some View {
        GridRow {
            Text(r.group == "sweep" ? r.label.replacingOccurrences(of: " calls", with: "").replacingOccurrences(of: " call", with: "") : r.label)
                .lineLimit(1)
                .minimumScaleFactor(0.7)
                .gridColumnAlignment(.leading)
            ns(r.swiftInline, r.running).foregroundStyle(.blue)
            ns(r.rustPerCall, r.running).foregroundStyle(.orange)
            if r.hasBatched { ns(r.rustBatched, r.running).foregroundStyle(.orange) } else { Text("—").foregroundStyle(.tertiary) }
            if let o = r.overheadNs { Text(fmt(o)).fontWeight(.semibold) } else { Text("—").foregroundStyle(.tertiary) }
            checksum(r.checksumMatch)
        }
    }

    private func ns(_ s: FfiSample?, _ running: Bool) -> some View {
        Group {
            if let s { Text(fmt(s.nsPerCall)) } else if running { ProgressView().controlSize(.mini) } else { Text("—").foregroundStyle(.tertiary) }
        }
    }

    private func fmt(_ ns: Double) -> String {
        ns >= 100 ? String(format: "%.0f", ns) : ns >= 10 ? String(format: "%.1f", ns) : String(format: "%.2f", ns)
    }

    @ViewBuilder
    private func checksum(_ ok: Bool?) -> some View {
        switch ok {
        case .some(true): Image(systemName: "checkmark.seal.fill").foregroundStyle(.green)
        case .some(false): Image(systemName: "xmark.seal.fill").foregroundStyle(.red)
        case .none: Image(systemName: "seal").foregroundStyle(.tertiary)
        }
    }

    // MARK: Chart

    struct Point: Identifiable {
        let id: String
        let calls: Double
        let ns: Double
        let series: String
    }

    @ViewBuilder
    private func chart(_ rows: [FfiRow]) -> some View {
        let pts: [Point] = rows.flatMap { r -> [Point] in
            var out: [Point] = []
            if let s = r.swiftInline { out.append(Point(id: r.id + "i", calls: Double(r.calls), ns: s.nsPerCall, series: "Swift inline")) }
            if let s = r.rustPerCall { out.append(Point(id: r.id + "p", calls: Double(r.calls), ns: s.nsPerCall, series: "Rust per call")) }
            if let s = r.rustBatched { out.append(Point(id: r.id + "b", calls: Double(r.calls), ns: s.nsPerCall, series: "Rust batched")) }
            return out
        }
        VStack(alignment: .leading, spacing: 6) {
            Text("ns per operation vs calls (log–log)").font(.subheadline.weight(.semibold))
            if pts.isEmpty {
                Text("Run #37 to see where the crossing cost dominates.")
                    .font(.caption).foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, minHeight: 120)
            } else {
                Chart(pts) { p in
                    LineMark(x: .value("Calls", p.calls), y: .value("ns/op", p.ns))
                        .foregroundStyle(by: .value("Variant", p.series))
                        .symbol(by: .value("Variant", p.series))
                }
                .chartXScale(type: .log)
                .chartYScale(type: .log)
                .chartForegroundStyleScale([
                    "Swift inline": Color.blue,
                    "Rust per call": Color.orange,
                    "Rust batched": Color.red,
                ])
                .chartXAxis {
                    AxisMarks(values: [1, 10, 100, 1000, 10000, 100_000]) { v in
                        AxisGridLine()
                        AxisValueLabel {
                            if let d = v.as(Double.self) { Text(d >= 1000 ? "\(Int(d / 1000))k" : "\(Int(d))") }
                        }
                    }
                }
                .frame(height: 190)
            }
        }
    }
}
