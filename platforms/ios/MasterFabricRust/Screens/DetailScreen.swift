import SwiftUI

/// 2. sayfa: detay. Favori kuralı ve ilgili öğeler Rust'ta hesaplanır.
struct DetailScreen: View {
    @Environment(CoreBridge.self) private var bridge
    let detail: DetailState

    var body: some View {
        let item = detail.item
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                HStack(spacing: 16) {
                    IconTile(symbol: item.symbol, color: color(for: item.category), size: 64)
                    VStack(alignment: .leading, spacing: 4) {
                        Text(item.category.uppercased())
                            .font(.caption.weight(.bold))
                            .foregroundStyle(color(for: item.category))
                        Text(item.title).font(.title2.bold())
                        Text(item.subtitle).font(.subheadline).foregroundStyle(.secondary)
                    }
                }

                Text(item.body).font(.body)

                ScrollView(.horizontal, showsIndicators: false) {
                    HStack {
                        ForEach(item.tags, id: \.self) { tag in
                            Text("#\(tag)")
                                .font(.caption.monospaced())
                                .padding(.horizontal, 10).padding(.vertical, 6)
                                .background(.orange.opacity(0.12), in: Capsule())
                        }
                    }
                }

                Button {
                    bridge.dispatch(.toggleFavorite(itemId: item.id))
                } label: {
                    Label(detail.favoriteLabel, systemImage: item.favorite ? "star.slash" : "star.fill")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)

                if !detail.related.isEmpty {
                    VStack(alignment: .leading, spacing: 10) {
                        Text("Aynı kategoriden").font(.headline)
                        ForEach(detail.related, id: \.id) { row in
                            Button {
                                bridge.dispatch(.openDetail(itemId: row.id))
                            } label: {
                                ItemRowView(row: row)
                                    .padding(12)
                                    .background(.background.secondary, in: RoundedRectangle(cornerRadius: 14))
                            }
                            .buttonStyle(.plain)
                        }
                    }
                }

                Text("Bu ekranın tüm verisi Rust `DetailViewModel`'den geliyor · \(detail.key)")
                    .font(.caption2.monospaced())
                    .foregroundStyle(.tertiary)
            }
            .padding()
        }
        .navigationTitle(item.title)
        .navigationBarTitleDisplayMode(.inline)
    }
}
