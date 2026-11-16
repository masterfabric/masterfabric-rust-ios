import Charts
import SwiftUI

/// Benchmark screen. Test list, categories, order, progress, ratios and verdicts live in Rust;
/// measurements run on a background thread (Rust single-thread, Rust rayon, then the same
/// algorithm in Swift). 36 tests in 8 categories; run all, one category, or one test.
struct BenchmarkScreen: View {
    @Environment(CoreBridge.self) private var bridge
    @State private var chartCategory = "Image"

    var body: some View {
        let b = bridge.benchmark
        let categories = orderedCategories(b.rows)
        ScrollViewReader { proxy in
            ScrollView {
                VStack(alignment: .leading, spacing: 14) {
                    EnvironmentCard(bench: b)
                    controls(b)
                    CategoryRunGrid(bench: b, categories: categories).id("categories")
                    if let summary = b.summary {
                        SummaryCard(summary: summary, totalMs: b.lastTotalMs)
                    }
                    SectionTitle(title: "Rendered output: Rust vs Swift", symbol: "sparkles.tv").id("media")
                    MediaPreviewCard()
                    SectionTitle(title: "Results table", symbol: "tablecells").id("table")
                    ResultsTable(rows: b.rows, categories: categories, threads: b.rustThreads)
                    SectionTitle(title: "Charts", symbol: "chart.bar.xaxis").id("charts")
                    RatioChartCard(rows: b.rows)
                    DurationChartCard(rows: b.rows, categories: categories, selection: $chartCategory)
                    SectionTitle(title: "App performance", symbol: "gauge.with.dots.needle.67percent").id("perf")
                    AppPerfCard()
                    ForEach(categories, id: \.self) { cat in
                        CategoryHeader(category: cat, rows: b.rows.filter { $0.category == cat }, running: b.running) {
                            bridge.dispatch(.runCategory(category: cat))
                        }
                        .id("cat-" + cat)
                        LazyVGrid(columns: [GridItem(.adaptive(minimum: 330), spacing: 12)], spacing: 12) {
                            ForEach(b.rows.filter { $0.category == cat }, id: \.id) { row in
                                BenchCard(row: row, threads: b.rustThreads, disabled: b.running) {
                                    bridge.dispatch(.runBenchmark(id: row.id))
                                }
                                .id(row.id)
                            }
                        }
                    }
                    Text("Each test: 1 warm-up run + the shown number of measured runs. Ratio = Swift avg / Rust single-thread avg. The Rust rayon variant is shown separately and is not part of the ratio. Checksums confirm both languages (and the parallel variant) produce the same result.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 12)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .background(Color(.systemGroupedBackground))
            .onChange(of: b.running) { _, running in
                // Debug/automation: `-scrollTo <target>` scrolls there when the run finishes.
                if !running, let target = UserDefaults.standard.string(forKey: "scrollTo") {
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.8) {
                        proxy.scrollTo(target, anchor: .top)
                    }
                }
            }
        }
        .navigationTitle("Benchmark")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button("Reset") { bridge.dispatch(.resetBenchmarks) }
                    .disabled(b.running)
            }
        }
    }

    @ViewBuilder
    private func controls(_ b: BenchmarkState) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Button {
                bridge.dispatch(.runAllBenchmarks)
            } label: {
                Label(b.running ? "Running…" : "Run all (\(b.rows.count) tests)",
                      systemImage: b.running ? "hourglass" : "play.fill")
                    .font(.headline)
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
            .disabled(b.running)
            .accessibilityIdentifier("run-all")

            if b.running || b.progress > 0 {
                ProgressView(value: b.progress) {
                    HStack {
                        Text(b.currentLabel ?? (b.running ? "Preparing…" : "Done"))
                        Spacer()
                        Text("\(Int(b.progress * 100))%").monospacedDigit()
                    }
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
                .tint(.orange)
            }
        }
    }
}

func orderedCategories(_ rows: [BenchRow]) -> [String] {
    var out: [String] = []
    for r in rows where !out.contains(r.category) { out.append(r.category) }
    return out
}

func shortName(_ id: String, fallback: String = "") -> String {
    switch id {
    case "raytrace": return "Ray tracing"
    case "raster": return "Rasterization"
    case "image": return "Image pipeline"
    case "video": return "Video processing"
    case "mesh": return "3D mesh"
    case "fib": return "Fibonacci"
    case "sort": return "Sort 1M"
    case "sha256": return "SHA-256"
    case "json": return "JSON"
    case "sieve": return "Prime sieve"
    case "matmul": return "Matrix"
    case "hashmap": return "HashMap"
    default: return fallback
    }
}

func categoryColor(_ c: String) -> Color {
    switch c {
    case "Image": return .pink
    case "Video": return .purple
    case "3D": return .indigo
    case "Math": return .blue
    case "Data": return .teal
    case "Text": return .green
    case "Crypto": return .orange
    case "Algorithms": return .red
    default: return .gray
    }
}

func categorySymbol(_ c: String) -> String {
    switch c {
    case "Image": return "photo.fill"
    case "Video": return "film.fill"
    case "3D": return "cube.fill"
    case "Math": return "function"
    case "Data": return "cylinder.split.1x2.fill"
    case "Text": return "textformat"
    case "Crypto": return "lock.fill"
    case "Algorithms": return "point.3.connected.trianglepath.dotted"
    default: return "cpu"
    }
}

let allCategories = ["Image", "Video", "3D", "Math", "Data", "Text", "Crypto", "Algorithms"]

/// Geometric mean of Swift/Rust ratios for the measured rows.
func geomean(_ rows: [BenchRow]) -> Double? {
    let r = rows.compactMap(\.ratio)
    guard !r.isEmpty else { return nil }
    return exp(r.map { log($0) }.reduce(0, +) / Double(r.count))
}

/// One button per category: run it, see its geomean.
private struct CategoryRunGrid: View {
    @Environment(CoreBridge.self) private var bridge
    let bench: BenchmarkState
    let categories: [String]

    var body: some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 160), spacing: 8)], spacing: 8) {
            ForEach(categories, id: \.self) { cat in
                let rows = bench.rows.filter { $0.category == cat }
                Button {
                    bridge.dispatch(.runCategory(category: cat))
                } label: {
                    HStack(spacing: 8) {
                        Image(systemName: categorySymbol(cat))
                            .font(.subheadline.weight(.semibold))
                            .foregroundStyle(.white)
                            .frame(width: 28, height: 28)
                            .background(categoryColor(cat).gradient, in: RoundedRectangle(cornerRadius: 7))
                        VStack(alignment: .leading, spacing: 1) {
                            Text(cat).font(.subheadline.weight(.semibold)).foregroundStyle(.primary)
                            Text(subtitle(rows)).font(.caption2.monospacedDigit()).foregroundStyle(.secondary)
                        }
                        Spacer(minLength: 2)
                        Image(systemName: "play.fill").font(.caption).foregroundStyle(categoryColor(cat))
                    }
                    .padding(8)
                    .background(.background, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
                }
                .buttonStyle(.plain)
                .disabled(bench.running)
                .accessibilityIdentifier("run-category-" + cat)
            }
        }
    }

    private func subtitle(_ rows: [BenchRow]) -> String {
        if let g = geomean(rows) { return "\(rows.count) tests · " + String(format: "%.2f×", g) }
        return "\(rows.count) tests"
    }
}

/// Section header for one category with its own Run button and geomean.
private struct CategoryHeader: View {
    let category: String
    let rows: [BenchRow]
    let running: Bool
    let run: () -> Void

    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            Label(category, systemImage: categorySymbol(category))
                .font(.title3.weight(.bold))
                .foregroundStyle(categoryColor(category))
            Text("\(rows.count) tests").font(.caption).foregroundStyle(.secondary)
            if let g = geomean(rows) {
                Text(String(format: "geomean %.2f×", g))
                    .font(.caption.weight(.semibold).monospacedDigit())
                    .foregroundStyle(ratioColor(g))
            }
            Spacer()
            Button(action: run) {
                Label("Run", systemImage: "play.fill").font(.subheadline.weight(.semibold))
            }
            .buttonStyle(.bordered)
            .tint(categoryColor(category))
            .disabled(running)
        }
        .padding(.top, 6)
    }
}

struct SectionTitle: View {
    let title: String
    let symbol: String
    var body: some View {
        Label(title, systemImage: symbol)
            .font(.title3.weight(.bold))
            .padding(.top, 6)
    }
}

struct Card<Content: View>: View {
    @ViewBuilder var content: Content
    var body: some View {
        content
            .padding(14)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(.background, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }
}

private struct EnvironmentCard: View {
    let bench: BenchmarkState

    var body: some View {
        Card {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Image(systemName: DeviceInfo.isSimulator ? "macbook.and.iphone" : "iphone")
                    Text("\(DeviceInfo.deviceName) · \(DeviceInfo.os)").font(.subheadline.weight(.semibold))
                }
                ViewThatFits(in: .horizontal) {
                    HStack(spacing: 6) { chips }
                    VStack(alignment: .leading, spacing: 6) { chips }
                }
                Text("\(DeviceInfo.machine) · \(bench.coreInfo)")
                    .font(.caption2.monospaced()).foregroundStyle(.tertiary)
                if DeviceInfo.isSimulator || DeviceInfo.swiftBuild == "Debug" || bench.rustProfile != "release" {
                    Label(caveat, systemImage: "exclamationmark.triangle.fill")
                        .font(.caption)
                        .foregroundStyle(.orange)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
        }
    }

    @ViewBuilder private var chips: some View {
        Chip(text: "Swift \(DeviceInfo.swiftBuild)", warn: DeviceInfo.swiftBuild == "Debug")
        Chip(text: "Rust \(bench.rustProfile)", warn: bench.rustProfile != "release")
        Chip(text: "\(DeviceInfo.cores) cores · \(DeviceInfo.memoryGB)")
    }

    private var caveat: String {
        var parts: [String] = []
        if DeviceInfo.isSimulator { parts.append("Simulator: the measured CPU is the Mac's, not a real iPhone.") }
        if DeviceInfo.swiftBuild == "Debug" { parts.append("Swift Debug build: unoptimized, Swift results will be very slow.") }
        if bench.rustProfile != "release" { parts.append("Rust debug build: results are meaningless.") }
        return parts.joined(separator: " ")
    }
}

private struct Chip: View {
    let text: String
    var warn = false
    var body: some View {
        Text(text)
            .font(.caption2.weight(.semibold))
            .lineLimit(1)
            .fixedSize()
            .padding(.horizontal, 8).padding(.vertical, 4)
            .background((warn ? Color.orange : Color.secondary).opacity(0.15), in: Capsule())
            .foregroundStyle(warn ? .orange : .primary)
    }
}

/// Rust and Swift ray tracer output side by side + summary of the 3D mesh generated in Rust.
private struct MediaPreviewCard: View {
    @Environment(CoreBridge.self) private var bridge
    @State private var rustImage: CGImage?
    @State private var swiftImage: CGImage?
    @State private var diffPixels: Int?
    @State private var mesh: MeshSummary?
    @State private var rendering = false

    private let size = 256

    var body: some View {
        Card {
            VStack(alignment: .leading, spacing: 12) {
                HStack(spacing: 12) {
                    preview(title: "Rust", color: .orange, image: rustImage)
                    preview(title: "Swift", color: .blue, image: swiftImage)
                }
                if let diffPixels {
                    Label(diffPixels == 0 ? "Both outputs are pixel-identical (\(size)×\(size))"
                                          : "\(diffPixels) pixels differ (floating-point rounding)",
                          systemImage: diffPixels == 0 ? "checkmark.seal.fill" : "exclamationmark.triangle.fill")
                        .font(.caption)
                        .foregroundStyle(diffPixels == 0 ? .green : .orange)
                }
                if let mesh {
                    HStack(spacing: 10) {
                        Image(systemName: "cube.transparent.fill").foregroundStyle(.purple)
                        Text("Rust 3D mesh: \(mesh.vertices.formatted()) vertices · \(mesh.triangles.formatted()) triangles · OBJ \(ByteCountFormatter.string(fromByteCount: Int64(mesh.objBytes), countStyle: .file))")
                            .font(.caption)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                }
                Button {
                    Task { await render() }
                } label: {
                    Label(rendering ? "Rendering…" : "Render again", systemImage: "arrow.clockwise")
                        .font(.subheadline)
                }
                .buttonStyle(.bordered)
                .disabled(rendering)
            }
        }
        .task { if rustImage == nil { await render() } }
    }

    private func preview(title: String, color: Color, image: CGImage?) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("\(title) ray tracer").font(.caption.weight(.bold)).foregroundStyle(color)
            ZStack {
                RoundedRectangle(cornerRadius: 12).fill(Color.secondary.opacity(0.1))
                if let image {
                    Image(decorative: image, scale: 1)
                        .resizable()
                        .interpolation(.medium)
                        .clipShape(RoundedRectangle(cornerRadius: 12))
                } else {
                    ProgressView()
                }
            }
            .aspectRatio(1, contentMode: .fit)
        }
        .frame(maxWidth: .infinity)
    }

    private func render() async {
        rendering = true
        let core = bridge.core
        let n = size
        let result = await Task.detached(priority: .userInitiated) { () -> ([UInt8], [UInt8], MeshSummary) in
            let rust = [UInt8](core.raytraceImage(width: UInt32(n), height: UInt32(n)))
            let swift = SwiftMedia.raytrace(n, n)
            let mesh = core.generateMesh(levels: 5)
            return (rust, swift, mesh)
        }.value
        rustImage = makeImage(result.0, n, n)
        swiftImage = makeImage(result.1, n, n)
        var diff = 0
        for p in stride(from: 0, to: min(result.0.count, result.1.count), by: 4)
        where result.0[p] != result.1[p] || result.0[p + 1] != result.1[p + 1] || result.0[p + 2] != result.1[p + 2] {
            diff += 1
        }
        diffPixels = diff
        mesh = result.2
        rendering = false
    }
}

func makeImage(_ rgba: [UInt8], _ w: Int, _ h: Int) -> CGImage? {
    guard rgba.count == w * h * 4, let provider = CGDataProvider(data: Data(rgba) as CFData) else { return nil }
    return CGImage(width: w, height: h, bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: w * 4,
                   space: CGColorSpace(name: CGColorSpace.sRGB)!,
                   bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.noneSkipLast.rawValue),
                   provider: provider, decode: nil, shouldInterpolate: true, intent: .defaultIntent)
}

/// Summary data table: all tests, grouped by category.
private struct ResultsTable: View {
    let rows: [BenchRow]
    let categories: [String]
    let threads: UInt32

    var body: some View {
        Card {
            VStack(alignment: .leading, spacing: 8) {
            Grid(alignment: .leading, horizontalSpacing: 12, verticalSpacing: 9) {
                GridRow {
                    Text("Test")
                    Text("Rust").gridColumnAlignment(.trailing)
                    Text("Rayon").gridColumnAlignment(.trailing)
                    Text("Swift").gridColumnAlignment(.trailing)
                    Text("Ratio").gridColumnAlignment(.trailing)
                }
                .font(.subheadline.weight(.bold))
                .foregroundStyle(.secondary)
                ForEach(categories, id: \.self) { cat in
                    GridRow {
                        Text(cat.uppercased())
                            .font(.caption.weight(.heavy))
                            .foregroundStyle(categoryColor(cat))
                            .gridCellColumns(5)
                            .padding(.top, 4)
                    }
                    Divider().gridCellUnsizedAxes(.horizontal)
                    ForEach(rows.filter { $0.category == cat }, id: \.id) { row in
                        GridRow {
                            HStack(spacing: 4) {
                                Text(shortName(row.id, fallback: row.name))
                                if row.checksumMatch == false {
                                    Image(systemName: "xmark.octagon.fill").foregroundStyle(.red)
                                }
                            }
                            .lineLimit(1)
                            .minimumScaleFactor(0.8)
                            msText(row.rust, active: row.status == .runningRust && row.rust == nil)
                            if row.hasParallel {
                                msText(row.rustParallel, active: row.status == .runningRust && row.rust != nil)
                                    .foregroundStyle(.brown)
                            } else {
                                Text("n/a").foregroundStyle(.tertiary)
                            }
                            msText(row.swift, active: row.status == .runningSwift)
                            ratioText(row)
                        }
                        .font(.footnote)
                    }
                }
            }
            Text("Rust = single thread · Rayon = Rust on \(threads) threads (not in ratio) · Ratio = Swift / Rust")
                .font(.caption2).foregroundStyle(.secondary)
            }
        }
    }

    @ViewBuilder
    private func msText(_ m: BenchMeasurement?, active: Bool) -> some View {
        if let m {
            Text(formatMs(m.avgMs)).monospacedDigit()
        } else if active {
            ProgressView().controlSize(.mini)
        } else {
            Text("—").foregroundStyle(.tertiary)
        }
    }

    @ViewBuilder
    private func ratioText(_ row: BenchRow) -> some View {
        if let r = row.ratio {
            Text(String(format: "%.2f×", r))
                .monospacedDigit()
                .fontWeight(.semibold)
                .foregroundStyle(ratioColor(r))
        } else {
            Text("—").foregroundStyle(.tertiary)
        }
    }
}

func ratioColor(_ r: Double) -> Color { r > 1.05 ? .orange : (r < 0.95 ? .blue : .secondary) }

/// Swift Charts: Swift/Rust ratio per test (1× = equal).
private struct RatioChartCard: View {
    let rows: [BenchRow]
    var body: some View {
        let done = rows.filter { $0.ratio != nil }
        Card {
            VStack(alignment: .leading, spacing: 8) {
                Text("Rust vs Swift (ratio)").font(.headline)
                Text("Bars right of 1× mean Rust is faster (single thread vs single thread).").font(.caption).foregroundStyle(.secondary)
                if done.isEmpty {
                    EmptyChartHint()
                } else {
                    Chart {
                        ForEach(done, id: \.id) { row in
                            BarMark(x: .value("Ratio", row.ratio ?? 0),
                                    y: .value("Test", shortName(row.id, fallback: row.name)))
                                .foregroundStyle(by: .value("Category", row.category))
                                .annotation(position: .trailing, alignment: .leading) {
                                    Text(String(format: "%.2f×", row.ratio ?? 0))
                                        .font(.caption2.monospacedDigit())
                                        .foregroundStyle(.secondary)
                                }
                        }
                        RuleMark(x: .value("Equal", 1.0))
                            .lineStyle(StrokeStyle(lineWidth: 1, dash: [4, 3]))
                            .foregroundStyle(.secondary)
                    }
                    .chartForegroundStyleScale(domain: allCategories, range: allCategories.map(categoryColor))
                    .chartLegend(position: .bottom)
                    .frame(height: CGFloat(done.count) * 30 + 50)
                }
            }
        }
    }
}

/// Swift Charts: Rust, Rust (rayon) and Swift side by side in ms for the selected category.
private struct DurationChartCard: View {
    let rows: [BenchRow]
    let categories: [String]
    @Binding var selection: String

    struct Point: Identifiable {
        let id: String
        let test: String
        let lang: String
        let ms: Double
    }

    var body: some View {
        let points: [Point] = rows.filter { $0.category == selection }.flatMap { r -> [Point] in
            var out: [Point] = []
            let name = shortName(r.id, fallback: r.name)
            if let m = r.rust { out.append(Point(id: r.id + "r", test: name, lang: "Rust", ms: m.avgMs)) }
            if let m = r.rustParallel { out.append(Point(id: r.id + "p", test: name, lang: "Rust (rayon)", ms: m.avgMs)) }
            if let m = r.swift { out.append(Point(id: r.id + "s", test: name, lang: "Swift", ms: m.avgMs)) }
            return out
        }
        Card {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text("Duration (ms, lower is better)").font(.headline)
                    Spacer()
                }
                Picker("Category", selection: $selection) {
                    ForEach(categories, id: \.self) { Text($0).tag($0) }
                }
                .pickerStyle(.menu)
                if points.isEmpty {
                    EmptyChartHint()
                } else {
                    Chart(points) { p in
                        BarMark(x: .value("ms", p.ms), y: .value("Test", p.test))
                            .foregroundStyle(by: .value("Language", p.lang))
                            .position(by: .value("Language", p.lang))
                    }
                    .chartForegroundStyleScale(["Rust": Color.orange, "Rust (rayon)": Color.brown, "Swift": Color.blue])
                    .chartLegend(position: .bottom)
                    .frame(height: CGFloat(Set(points.map(\.test)).count) * 56 + 50)
                }
            }
        }
    }
}

private struct EmptyChartHint: View {
    var body: some View {
        Label("Run the tests to see charts", systemImage: "chart.bar")
            .font(.subheadline)
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity, minHeight: 80)
    }
}

private struct SummaryCard: View {
    let summary: String
    let totalMs: Double?
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Label(summary, systemImage: "chart.bar.xaxis").font(.subheadline.weight(.semibold))
            if let totalMs {
                Text("Total time (incl. warm-up, all variants): \(String(format: "%.2f", totalMs / 1000)) s")
                    .font(.caption).foregroundStyle(.secondary)
            }
            Text("Ratio > 1: Rust faster (orange) · < 1: Swift faster (blue)")
                .font(.caption2).foregroundStyle(.secondary)
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.orange.opacity(0.10), in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }
}

private struct BenchCard: View {
    let row: BenchRow
    let threads: UInt32
    let disabled: Bool
    let run: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 12) {
                IconTile(symbol: row.symbol, color: categoryColor(row.category), size: 36)
                VStack(alignment: .leading, spacing: 2) {
                    Text(row.name).font(.subheadline.weight(.semibold))
                    Text("\(row.description) · \(row.iterations)× measured")
                        .font(.caption).foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Spacer(minLength: 4)
                Button(action: run) {
                    Image(systemName: "play.circle.fill").font(.title2)
                }
                .disabled(disabled)
                .accessibilityLabel("Run \(row.name)")
            }

            HStack(spacing: 10) {
                LangColumn(lang: "Rust", color: .orange, m: row.rust, active: row.status == .runningRust)
                LangColumn(lang: "Swift", color: .blue, m: row.swift, active: row.status == .runningSwift)
            }

            if let r = row.rust, let s = row.swift {
                RatioBar(rust: r.avgMs, swift: s.avgMs)
            }

            if row.hasParallel {
                HStack(spacing: 6) {
                    Image(systemName: "square.stack.3d.up.fill").foregroundStyle(.brown)
                    if let p = row.rustParallel {
                        Text("Rust ×\(threads) threads (rayon): \(formatMs(p.avgMs))")
                            .monospacedDigit()
                        if let sp = row.parallelSpeedup {
                            Text(String(format: "· %.1f× vs 1 thread", sp)).foregroundStyle(.secondary)
                        }
                    } else {
                        Text("Rust ×\(threads) threads (rayon): —").foregroundStyle(.secondary)
                    }
                }
                .font(.caption)
                .lineLimit(1)
                .minimumScaleFactor(0.8)
            }

            HStack {
                StatusBadge(status: row.status)
                Spacer()
                if let v = row.verdict { Text(v).font(.caption.weight(.semibold)) }
                if let ok = row.checksumMatch {
                    Label(ok ? "checksum match" : "checksum mismatch", systemImage: ok ? "checkmark.seal.fill" : "xmark.seal.fill")
                        .font(.caption2)
                        .foregroundStyle(ok ? .green : .red)
                }
            }
        }
        .padding(14)
        .background(.background, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }
}

private struct LangColumn: View {
    let lang: String
    let color: Color
    let m: BenchMeasurement?
    let active: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(lang).font(.caption2.weight(.bold)).foregroundStyle(color)
            if let m {
                Text(formatMs(m.avgMs)).font(.title3.weight(.semibold).monospacedDigit())
                Text("best \(formatMs(m.minMs)) · \(m.iterations)×")
                    .font(.caption2.monospacedDigit()).foregroundStyle(.secondary)
            } else if active {
                ProgressView().frame(height: 28)
            } else {
                Text("—").font(.title3).foregroundStyle(.tertiary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(10)
        .background(color.opacity(0.08), in: RoundedRectangle(cornerRadius: 10))
    }
}

private struct RatioBar: View {
    let rust: Double
    let swift: Double
    var body: some View {
        GeometryReader { geo in
            let maxV = max(rust, swift)
            VStack(alignment: .leading, spacing: 3) {
                Capsule().fill(.orange).frame(width: max(4, geo.size.width * rust / maxV), height: 6)
                Capsule().fill(.blue).frame(width: max(4, geo.size.width * swift / maxV), height: 6)
            }
        }
        .frame(height: 15)
    }
}

private struct StatusBadge: View {
    let status: BenchStatus
    var body: some View {
        let (text, color): (String, Color) = {
            switch status {
            case .idle: return ("Idle", .secondary)
            case .queued: return ("Queued", .secondary)
            case .runningRust: return ("Measuring Rust", .orange)
            case .runningSwift: return ("Measuring Swift", .blue)
            case .done: return ("Done", .green)
            case .failed(let message): return (message, .red)
            }
        }()
        Text(text)
            .font(.caption2.weight(.semibold))
            .padding(.horizontal, 8).padding(.vertical, 3)
            .background(color.opacity(0.15), in: Capsule())
            .foregroundStyle(color)
    }
}

func formatMs(_ ms: Double) -> String {
    if ms >= 100 { return String(format: "%.0f ms", ms) }
    if ms >= 10 { return String(format: "%.1f ms", ms) }
    return String(format: "%.2f ms", ms)
}

/// App-level performance: launch time, dispatch → render latency, Rust dispatch cost.
/// Values are recorded into Rust (`recordPerf`) and rendered from Rust state.
struct AppPerfCard: View {
    @Environment(CoreBridge.self) private var bridge

    var body: some View {
        Card {
            VStack(alignment: .leading, spacing: 10) {
                if bridge.perf.isEmpty {
                    Text("No measurements yet.").font(.subheadline).foregroundStyle(.secondary)
                }
                ForEach(bridge.perf, id: \.id) { m in
                    HStack(alignment: .firstTextBaseline) {
                        VStack(alignment: .leading, spacing: 1) {
                            Text(m.label).font(.subheadline)
                            Text(m.detail).font(.caption2).foregroundStyle(.secondary)
                                .fixedSize(horizontal: false, vertical: true)
                        }
                        Spacer(minLength: 8)
                        Text(formatPerf(m)).font(.subheadline.weight(.semibold).monospacedDigit())
                    }
                }
                Button {
                    Task { await bridge.measureAndRecordLatency() }
                } label: {
                    Label(bridge.measuringLatency ? "Measuring…" : "Measure UI latency", systemImage: "stopwatch")
                        .font(.subheadline)
                }
                .buttonStyle(.bordered)
                .disabled(bridge.measuringLatency || bridge.benchmark.running)
                .accessibilityIdentifier("measure-latency")
            }
        }
    }
}

func formatPerf(_ m: PerfMetric) -> String {
    switch m.unit {
    case "µs": return m.value >= 1000 ? String(format: "%.2f ms", m.value / 1000) : String(format: "%.1f µs", m.value)
    case "ms": return String(format: "%.1f ms", m.value)
    default: return String(format: "%.2f %@", m.value, m.unit)
    }
}
