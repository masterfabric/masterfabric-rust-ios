import SwiftUI

/// The navigation stack comes from Rust; SwiftUI only renders a screen per key.
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
        } else if key == "info" {
            InfoScreen()
        } else if let detail = bridge.detail(for: key) {
            DetailScreen(detail: detail)
        } else {
            ContentUnavailableView("Screen not found", systemImage: "questionmark.square.dashed",
                                   description: Text(key))
        }
    }
}
