import Charts
import SwiftUI

/// "What can you build with Rust?" infographic screen. All content comes from the Rust
/// `InfoViewModel`; numbers in the chart are only this app's own benchmark measurements.
struct InfoScreen: View {
    @Environment(CoreBridge.self) private var bridge

    var body: some View {
        let info = bridge.info
        ScrollViewReader { proxy in
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Wordmark(style: .header).padding(.top, -8)
                Hero(title: info.title, intro: info.intro)

                SectionTitle(title: "This app's architecture", symbol: "arrow.triangle.2.circlepath")
                ArchitectureDiagram(steps: info.architecture)

                SectionTitle(title: "What you can build with Rust", symbol: "square.grid.2x2.fill").id("projects")
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 165), spacing: 12)], spacing: 12) {
                    ForEach(info.categories, id: \.id) { CategoryCard(category: $0) }
                }

                SectionTitle(title: "Measured on this device", symbol: "speedometer").id("measured")
                MeasuredChart(measured: info.measured, note: info.measuredNote) {
                    bridge.dispatch(.openBenchmark)
                }

                SectionTitle(title: "App performance", symbol: "gauge.with.dots.needle.67percent").id("perf")
                AppPerfCard()

                HStack(alignment: .top, spacing: 12) {
                    BulletCard(title: "Strengths", symbol: "checkmark.circle.fill", color: .green, items: info.strengths)
                    BulletCard(title: "Trade-offs", symbol: "exclamationmark.circle.fill", color: .orange, items: info.tradeoffs)
                }
            }
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .background(Color(.systemGroupedBackground))
        .onAppear {
            // Debug/automation: `-infoScrollTo projects|measured`
            if let target = UserDefaults.standard.string(forKey: "infoScrollTo") {
                DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) { proxy.scrollTo(target, anchor: .top) }
            }
        }
        }
        .navigationTitle("Rust guide")
        .navigationBarTitleDisplayMode(.inline)
    }
}

func tintColor(_ key: String) -> Color {
    switch key {
    case "orange": return .orange
    case "blue": return .blue
    case "green": return .green
    case "purple": return .purple
    case "teal": return .teal
    case "pink": return .pink
    case "indigo": return .indigo
    case "red": return .red
    case "mint": return .mint
    case "brown": return .brown
    case "cyan": return .cyan
    default: return .gray
    }
}

private struct Hero: View {
    let title: String
    let intro: String
    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 10) {
                Image(systemName: "gearshape.2.fill").font(.largeTitle)
                Text(title).font(.title2.bold())
            }
            Text(intro).font(.subheadline).fixedSize(horizontal: false, vertical: true)
        }
        .foregroundStyle(.white)
        .padding(18)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(LinearGradient(colors: [.orange, .red.opacity(0.85)], startPoint: .topLeading, endPoint: .bottomTrailing),
                    in: RoundedRectangle(cornerRadius: 20, style: .continuous))
    }
}

/// SwiftUI renders → dispatch(Action) → Rust state → partial ViewUpdate → SwiftUI again.
private struct ArchitectureDiagram: View {
    let steps: [ArchStep]

    var body: some View {
        Card {
            VStack(spacing: 0) {
                HStack(spacing: 14) {
                    Legend(color: .blue, text: "Swift (native UI)")
                    Legend(color: .orange, text: "Rust (core)")
                    Spacer(minLength: 0)
                }
                .padding(.bottom, 10)
                HStack(alignment: .center, spacing: 8) {
                    VStack(spacing: 0) {
                        ForEach(Array(steps.enumerated()), id: \.offset) { idx, step in
                            StepBox(step: step)
                            if idx < steps.count - 1 {
                                Image(systemName: "arrow.down")
                                    .font(.caption.weight(.bold))
                                    .foregroundStyle(.secondary)
                                    .frame(height: 18)
                            }
                        }
                    }
                    // Loop arrow: the last step feeds back into the first.
                    VStack(spacing: 4) {
                        Image(systemName: "arrow.uturn.up").font(.title3.weight(.semibold))
                        Text("loop").font(.caption2).foregroundStyle(.secondary)
                            .rotationEffect(.degrees(-90)).fixedSize().frame(width: 16, height: 40)
                    }
                    .foregroundStyle(.secondary)
                    .frame(width: 26)
                }
            }
        }
    }
}

private struct Legend: View {
    let color: Color
    let text: String
    var body: some View {
        HStack(spacing: 5) {
            RoundedRectangle(cornerRadius: 3).fill(color).frame(width: 12, height: 12)
            Text(text).font(.caption2)
        }
    }
}

private struct StepBox: View {
    let step: ArchStep
    var body: some View {
        let color: Color = step.layer == "rust" ? .orange : .blue
        HStack(spacing: 12) {
            Image(systemName: step.symbol)
                .font(.title3)
                .foregroundStyle(.white)
                .frame(width: 38, height: 38)
                .background(color.gradient, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            VStack(alignment: .leading, spacing: 2) {
                Text(step.title).font(.subheadline.weight(.semibold).monospaced())
                Text(step.detail).font(.caption).foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Spacer(minLength: 0)
        }
        .padding(10)
        .background(color.opacity(0.10), in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).stroke(color.opacity(0.35)))
    }
}

private struct CategoryCard: View {
    let category: ProjectCategory
    var body: some View {
        let color = tintColor(category.tint)
        VStack(alignment: .leading, spacing: 8) {
            Image(systemName: category.symbol)
                .font(.title2)
                .foregroundStyle(.white)
                .frame(width: 44, height: 44)
                .background(color.gradient, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
            Text(category.title).font(.headline).fixedSize(horizontal: false, vertical: true)
            Text(category.summary).font(.caption).foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            VStack(alignment: .leading, spacing: 3) {
                ForEach(category.examples, id: \.self) { ex in
                    Label(ex, systemImage: "checkmark")
                        .font(.caption2)
                        .labelStyle(TightLabel(color: color))
                }
            }
            Text(category.crates.joined(separator: " · "))
                .font(.caption2.monospaced())
                .foregroundStyle(color)
                .fixedSize(horizontal: false, vertical: true)
            Spacer(minLength: 0)
        }
        .padding(12)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(.background, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }
}

private struct TightLabel: LabelStyle {
    let color: Color
    func makeBody(configuration: Configuration) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 4) {
            configuration.icon.foregroundStyle(color)
            configuration.title
        }
    }
}

private struct MeasuredChart: View {
    let measured: [MeasuredRatio]
    let note: String
    let openBenchmark: () -> Void

    var body: some View {
        Card {
            VStack(alignment: .leading, spacing: 10) {
                Text(note).font(.caption).foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                if measured.isEmpty {
                    Button(action: openBenchmark) {
                        Label("Open benchmark", systemImage: "speedometer").frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                } else {
                    Chart {
                        ForEach(measured, id: \.benchId) { m in
                            BarMark(x: .value("Ratio", m.ratio), y: .value("Test", shortName(m.benchId, fallback: m.name)))
                                .foregroundStyle(by: .value("Category", m.category))
                                .annotation(position: .trailing, alignment: .leading) {
                                    Text(String(format: "%.2f×", m.ratio)).font(.caption2.monospacedDigit())
                                        .foregroundStyle(.secondary)
                                }
                        }
                        RuleMark(x: .value("Equal", 1.0))
                            .lineStyle(StrokeStyle(lineWidth: 1, dash: [4, 3]))
                            .foregroundStyle(.secondary)
                    }
                    .chartForegroundStyleScale(["Media": Color.pink, "General": Color.orange])
                    .chartLegend(position: .bottom)
                    .frame(height: CGFloat(measured.count) * 28 + 50)
                }
            }
        }
    }
}

private struct BulletCard: View {
    let title: String
    let symbol: String
    let color: Color
    let items: [String]
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(title, systemImage: symbol).font(.subheadline.weight(.bold)).foregroundStyle(color)
            ForEach(items, id: \.self) { item in
                Text("• \(item)").font(.caption).fixedSize(horizontal: false, vertical: true)
            }
            Spacer(minLength: 0)
        }
        .padding(12)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(.background, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }
}
