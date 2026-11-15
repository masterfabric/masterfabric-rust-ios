import SwiftUI

/// Page 1: list. Search, sorting and favorite rules live in Rust.
struct HomeScreen: View {
    @Environment(CoreBridge.self) private var bridge

    var body: some View {
        let home = bridge.home
        List {
            Section {
                VStack(spacing: 12) {
                    Wordmark(style: .header)
                    Text(home.title)
                        .font(.title2.weight(.bold))
                        .frame(maxWidth: .infinity)
                        .accessibilityIdentifier("home-title")
                    ArchitectureBanner(revision: bridge.revision)
                }
                .listRowInsets(EdgeInsets(top: 0, leading: 16, bottom: 12, trailing: 16))
            }

            Section {
                Button {
                    bridge.dispatch(.openBenchmark)
                } label: {
                    NavRow(symbol: "speedometer", color: .orange, title: "Benchmark: Rust vs Swift",
                           subtitle: "Media + general workloads · real measurements")
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("open-benchmark")

                Button {
                    bridge.dispatch(.openInfo)
                } label: {
                    NavRow(symbol: "lightbulb.max.fill", color: .indigo, title: "What can you build with Rust?",
                           subtitle: "Infographic: projects, architecture, measurements")
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("open-info")
            }

            Section {
                if let empty = home.emptyMessage {
                    ContentUnavailableView.search(text: empty)
                }
                ForEach(home.rows, id: \.id) { row in
                    Button {
                        bridge.dispatch(.openDetail(itemId: row.id))
                    } label: {
                        ItemRowView(row: row)
                    }
                    .buttonStyle(.plain)
                }
            } header: {
                Text("Rust projects · \(home.rows.count)/\(home.totalCount)"
                     + (home.favoriteCount > 0 ? " · ★ \(home.favoriteCount)" : ""))
            }
        }
        .navigationTitle(home.title)
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .principal) { Wordmark(style: .navigation) }
            ToolbarItem(placement: .topBarTrailing) {
                Button {
                    bridge.dispatch(.openInfo)
                } label: {
                    Image(systemName: "info.circle")
                }
                .accessibilityLabel("Rust guide")
            }
            ToolbarItem(placement: .topBarTrailing) {
                Button {
                    bridge.dispatch(.openBenchmark)
                } label: {
                    Image(systemName: "speedometer")
                }
                .accessibilityLabel("Benchmark")
            }
        }
        .searchable(text: Binding(
            get: { bridge.home.query },
            set: { bridge.dispatch(.setQuery(query: $0)) }
        ), prompt: "Search: embassy, mobile, robot…")
    }
}

private struct NavRow: View {
    let symbol: String
    let color: Color
    let title: String
    let subtitle: String
    var body: some View {
        HStack(spacing: 14) {
            IconTile(symbol: symbol, color: color)
            VStack(alignment: .leading, spacing: 2) {
                Text(title).font(.headline)
                Text(subtitle).font(.subheadline).foregroundStyle(.secondary)
            }
            Spacer()
            Image(systemName: "chevron.right").font(.footnote.weight(.semibold))
                .foregroundStyle(.tertiary)
        }
        .contentShape(Rectangle())
    }
}

struct ArchitectureBanner: View {
    let revision: UInt64
    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: "gearshape.2.fill")
                .font(.title2)
                .foregroundStyle(.orange)
            VStack(alignment: .leading, spacing: 4) {
                Text("Shared Rust core · native UI").font(.subheadline.weight(.semibold))
                Text("State, navigation, data and business rules live in the Rust core. SwiftUI only renders.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                Text("Rust state revision: #\(revision)")
                    .font(.caption2.monospaced())
                    .foregroundStyle(.tertiary)
            }
            Spacer(minLength: 0)
        }
        .padding(12)
        .background(.orange.opacity(0.08), in: RoundedRectangle(cornerRadius: 12, style: .continuous))
    }
}

struct ItemRowView: View {
    let row: ItemRow
    var body: some View {
        HStack(spacing: 14) {
            IconTile(symbol: row.symbol, color: color(for: row.category))
            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 6) {
                    Text(row.title).font(.headline).lineLimit(1)
                    if row.favorite {
                        Image(systemName: "star.fill").font(.caption).foregroundStyle(.yellow)
                    }
                }
                Text(row.subtitle).font(.subheadline).foregroundStyle(.secondary).lineLimit(1)
                Text(row.category.uppercased())
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(color(for: row.category))
            }
            Spacer(minLength: 0)
            Image(systemName: "chevron.right").font(.footnote.weight(.semibold)).foregroundStyle(.tertiary)
        }
        .contentShape(Rectangle())
        .padding(.vertical, 2)
    }
}

struct IconTile: View {
    let symbol: String
    let color: Color
    var size: CGFloat = 40
    var body: some View {
        Image(systemName: symbol)
            .font(.system(size: size * 0.45, weight: .semibold))
            .foregroundStyle(.white)
            .frame(width: size, height: size)
            .background(color.gradient, in: RoundedRectangle(cornerRadius: size * 0.25, style: .continuous))
    }
}

func color(for category: String) -> Color {
    switch category {
    case "Mobile": return .blue
    case "Embedded": return .green
    case "Robotics": return .purple
    case "Tools": return .teal
    case "Games": return .pink
    default: return .orange
    }
}
