import SwiftUI

@main
struct MasterFabricRustApp: App {
    @State private var bridge = CoreBridge()

    var body: some Scene {
        WindowGroup {
            RootView()
                .environment(bridge)
                .task { bridge.applyLaunchOptions() }
        }
    }
}
