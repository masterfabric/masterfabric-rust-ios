import SwiftUI

@main
struct MasterFabricRustApp: App {
    @State private var bridge = CoreBridge()

    var body: some Scene {
        WindowGroup {
            RootView()
                .environment(bridge)
                .onAppear {
                    if Perf.launchToFirstFrameMs == nil, let start = Perf.processStart {
                        Perf.launchToFirstFrameMs = (Perf.nowEpoch() - start) * 1000
                    }
                }
                .task { await bridge.applyLaunchOptions() }
        }
    }
}
