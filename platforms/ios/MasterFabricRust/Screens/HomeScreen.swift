import SwiftUI

/// 1. sayfa: liste. Arama, sıralama ve favori kuralları Rust'ta.
struct HomeScreen: View {
    @Environment(CoreBridge.self) private var bridge

    var body: some View {
        let home = bridge.state.home
        List {
            Section {
                ArchitectureBanner(revision: bridge.state.revision)
                    .listRowInsets(EdgeInsets(top: 8, leading: 16, bottom: 8, trailing: 16))
            }

            Section {
                Button {
                    bridge.dispatch(.openBenchmark)
                } label: {
                    HStack(spacing: 14) {
                        IconTile(symbol: "speedometer", color: .orange)
                        VStack(alignment: .leading, spacing: 2) {
                            Text("Benchmark: Rust vs Swift").font(.headline)
                            Text("7 test · cihaz üzerinde gerçek ölçüm")
                                .font(.subheadline).foregroundStyle(.secondary)
                        }
                        Spacer()
                        Image(systemName: "chevron.right").font(.footnote.weight(.semibold))
                            .foregroundStyle(.tertiary)
                    }
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("open-benchmark")
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
                Text("Rust ile projeler · \(home.rows.count)/\(home.totalCount)"
                     + (home.favoriteCount > 0 ? " · ★ \(home.favoriteCount)" : ""))
            }
        }
        .navigationTitle(home.title)
        .searchable(text: Binding(
            get: { bridge.state.home.query },
            set: { bridge.dispatch(.setQuery(query: $0)) }
        ), prompt: "Ara: embassy, mobil, robot…")
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button {
                    bridge.dispatch(.openBenchmark)
                } label: {
                    Image(systemName: "speedometer")
                }
                .accessibilityLabel("Benchmark")
            }
        }
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
                Text("Discord modeli").font(.subheadline.weight(.semibold))
                Text("Durum, navigasyon, veri ve iş kuralları Rust çekirdeğinde. SwiftUI yalnızca çiziyor.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                Text("Rust durum revizyonu: #\(revision)")
                    .font(.caption2.monospaced())
                    .foregroundStyle(.tertiary)
            }
        }
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
    case "Mobil": return .blue
    case "Gömülü": return .green
    case "Robotik": return .purple
    case "Araçlar": return .teal
    case "Oyun": return .pink
    default: return .orange
    }
}
