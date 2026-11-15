import SwiftUI

/// Benchmark ekranı. Test listesi, sıra, ilerleme, oranlar ve yorumlar Rust'ta;
/// ölçümler arka plan thread'inde (önce Rust, sonra aynı algoritmanın Swift'i) yapılır.
struct BenchmarkScreen: View {
    @Environment(CoreBridge.self) private var bridge

    var body: some View {
        let b = bridge.state.benchmark
        ScrollViewReader { proxy in
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                EnvironmentCard(bench: b)
                controls(b)
                ResultsTable(rows: b.rows)
                if let summary = b.summary {
                    SummaryCard(summary: summary, totalMs: b.lastTotalMs)
                }
                Text("Testler").font(.headline).padding(.top, 4)
                ForEach(b.rows, id: \.id) { row in
                    BenchCard(row: row, disabled: b.running) {
                        bridge.dispatch(.runBenchmark(id: row.id))
                    }
                    .id(row.id)
                }
                Text("Her test: 1 ısınma turu + gösterilen sayıda ölçüm. Oran = Swift ort. / Rust ort. Checksum iki dilin aynı sonucu ürettiğini doğrular.")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
            .padding()
        }
        .background(Color(.systemGroupedBackground))
        .onChange(of: b.running) { _, running in
            // Hata ayıklama/otomasyon: `-scrollTo <testId>` çalıştırma bitince o karta kaydırır.
            if !running, let target = UserDefaults.standard.string(forKey: "scrollTo") {
                withAnimation { proxy.scrollTo(target, anchor: .top) }
            }
        }
        }
        .navigationTitle("Benchmark")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button("Sıfırla") { bridge.dispatch(.resetBenchmarks) }
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
                Label(b.running ? "Çalışıyor…" : "Tümünü çalıştır", systemImage: b.running ? "hourglass" : "play.fill")
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
                        Text(b.currentLabel ?? (b.running ? "Hazırlanıyor…" : "Tamamlandı"))
                        Spacer()
                        Text("%\(Int(b.progress * 100))").monospacedDigit()
                    }
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
                .tint(.orange)
            }
        }
    }
}

private struct EnvironmentCard: View {
    let bench: BenchmarkState

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Image(systemName: DeviceInfo.isSimulator ? "macbook.and.iphone" : "iphone")
                Text("\(DeviceInfo.deviceName) · \(DeviceInfo.os)").font(.subheadline.weight(.semibold))
            }
            HStack(spacing: 6) {
                Chip(text: "Swift \(DeviceInfo.swiftBuild)", warn: DeviceInfo.swiftBuild == "Debug")
                Chip(text: "Rust \(bench.rustProfile)", warn: bench.rustProfile != "release")
                Chip(text: "\(DeviceInfo.cores) çekirdek · \(DeviceInfo.memoryGB)")
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
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding()
        .background(.background, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }

    private var caveat: String {
        var parts: [String] = []
        if DeviceInfo.isSimulator { parts.append("Simülatör: ölçülen işlemci Mac'in CPU'su, gerçek iPhone değil.") }
        if DeviceInfo.swiftBuild == "Debug" { parts.append("Swift Debug derlemesi: optimize değil, Swift sonuçları çok yavaş çıkar.") }
        if bench.rustProfile != "release" { parts.append("Rust debug derlemesi: sonuçlar anlamsız.") }
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

/// Özet veri tablosu: tüm testler tek bakışta.
private struct ResultsTable: View {
    let rows: [BenchRow]

    var body: some View {
        Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 8) {
            GridRow {
                Text("Test")
                Text("Rust").gridColumnAlignment(.trailing)
                Text("Swift").gridColumnAlignment(.trailing)
                Text("Oran").gridColumnAlignment(.trailing)
            }
            .font(.caption.weight(.bold))
            .foregroundStyle(.secondary)
            Divider()
            ForEach(rows, id: \.id) { row in
                GridRow {
                    HStack(spacing: 4) {
                        Text(shortName(row))
                        if row.checksumMatch == false {
                            Image(systemName: "xmark.octagon.fill").foregroundStyle(.red)
                        }
                    }
                    .lineLimit(1)
                    msText(row.rust, active: row.status == .runningRust)
                    msText(row.swift, active: row.status == .runningSwift)
                    ratioText(row)
                }
                .font(.footnote)
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.background, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }

    private func shortName(_ row: BenchRow) -> String {
        switch row.id {
        case "fib": return "Fibonacci"
        case "sort": return "Sıralama 1M"
        case "sha256": return "SHA-256"
        case "json": return "JSON"
        case "sieve": return "Asal elek"
        case "matmul": return "Matris"
        case "hashmap": return "HashMap"
        default: return row.name
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
                .foregroundStyle(r >= 1.05 ? Color.orange : (r <= 0.95 ? Color.blue : Color.secondary))
        } else {
            Text("—").foregroundStyle(.tertiary)
        }
    }
}

private struct SummaryCard: View {
    let summary: String
    let totalMs: Double?
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Label(summary, systemImage: "chart.bar.xaxis").font(.subheadline.weight(.semibold))
            if let totalMs {
                Text("Toplam süre (ısınma dahil, iki dil): \(String(format: "%.2f", totalMs / 1000)) sn")
                    .font(.caption).foregroundStyle(.secondary)
            }
            Text("Oran > 1: Rust daha hızlı (turuncu) · < 1: Swift daha hızlı (mavi)")
                .font(.caption2).foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding()
        .background(.orange.opacity(0.10), in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }
}

private struct BenchCard: View {
    let row: BenchRow
    let disabled: Bool
    let run: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 12) {
                IconTile(symbol: row.symbol, color: .orange, size: 34)
                VStack(alignment: .leading, spacing: 2) {
                    Text(row.name).font(.subheadline.weight(.semibold))
                    Text("\(row.description) · \(row.iterations)× ölçüm")
                        .font(.caption).foregroundStyle(.secondary)
                }
                Spacer(minLength: 4)
                Button(action: run) {
                    Image(systemName: "play.circle.fill").font(.title2)
                }
                .disabled(disabled)
                .accessibilityLabel("\(row.name) çalıştır")
            }

            HStack(spacing: 10) {
                LangColumn(lang: "Rust", color: .orange, m: row.rust, active: row.status == .runningRust)
                LangColumn(lang: "Swift", color: .blue, m: row.swift, active: row.status == .runningSwift)
            }

            if let r = row.rust, let s = row.swift {
                RatioBar(rust: r.avgMs, swift: s.avgMs)
            }

            HStack {
                StatusBadge(status: row.status)
                Spacer()
                if let v = row.verdict { Text(v).font(.caption.weight(.semibold)) }
                if let ok = row.checksumMatch {
                    Label(ok ? "checksum eşleşti" : "checksum farklı", systemImage: ok ? "checkmark.seal.fill" : "xmark.seal.fill")
                        .font(.caption2)
                        .foregroundStyle(ok ? .green : .red)
                }
            }
        }
        .padding()
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
                Text("en iyi \(formatMs(m.minMs)) · \(m.iterations)×")
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
            case .idle: return ("Bekliyor", .secondary)
            case .queued: return ("Sırada", .secondary)
            case .runningRust: return ("Rust ölçülüyor", .orange)
            case .runningSwift: return ("Swift ölçülüyor", .blue)
            case .done: return ("Tamamlandı", .green)
            case .failed(let message): return (message, .red)
            }
        }()
        Text(text)
            .font(.caption2.weight(.semibold))
            .lineLimit(1)
            .fixedSize()
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
