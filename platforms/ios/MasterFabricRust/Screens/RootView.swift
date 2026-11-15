import SwiftUI

/// Navigasyon yığını Rust'tan gelir; SwiftUI yalnızca anahtara göre ekranı çizer.
struct RootView: View {
    @Environment(CoreBridge.self) private var bridge

    var body: some View {
        NavigationStack(path: bridge.path) {
            HomeScreen()
                .navigationDestination(for: String.self) { key in
                    destination(for: key)
                }
        }
        .tint(.orange)
    }

    @ViewBuilder
    private func destination(for key: String) -> some View {
        if key == "benchmark" {
            BenchmarkScreen()
        } else if let detail = bridge.detail(for: key) {
            DetailScreen(detail: detail)
        } else {
            ContentUnavailableView("Ekran bulunamadı", systemImage: "questionmark.square.dashed",
                                   description: Text(key))
        }
    }
}
