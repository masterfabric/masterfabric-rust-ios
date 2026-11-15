import Foundation
import Observation
import SwiftUI

/// Rust durumunun SwiftUI tarafındaki aynası.
///
/// Burada iş mantığı yok: kullanıcı olayları `dispatch` ile Rust'a gider,
/// Rust yeni `AppViewState`'i `StateListener` ile geri gönderir, SwiftUI çizer.
@Observable
final class CoreBridge {
    private(set) var state: AppViewState
    /// Geri kaydırma animasyonu sırasında kaybolan ekranın içeriği boşalmasın diye son detaylar.
    private(set) var detailCache: [String: DetailState] = [:]

    @ObservationIgnored let core: AppCore
    @ObservationIgnored private var listener: Listener?
    @ObservationIgnored private let runner = SwiftRunner()

    init() {
        let core = AppCore()
        self.core = core
        self.state = core.state()
        let listener = Listener(bridge: self)
        self.listener = listener
        core.setListener(listener: listener)
        core.setSwiftRunner(runner: runner)
    }

    func dispatch(_ action: Action) {
        core.dispatch(action: action)
    }

    fileprivate func apply(_ new: AppViewState) {
        guard new.revision >= state.revision else { return } // sırası karışan bildirimleri at
        for d in new.details { detailCache[d.key] = d }
        state = new
    }

    // MARK: Navigasyon (yığın Rust'ta)

    /// SwiftUI `NavigationStack(path:)` bağlaması. Push'lar yalnızca Rust'tan gelir;
    /// kullanıcı kaydırarak/geri tuşuyla dönerse Rust'a `popTo` gönderilir.
    var path: Binding<[String]> {
        Binding(
            get: { self.state.stack.dropFirst().map(\.key) },
            set: { newValue in
                let current = self.state.stack.count - 1
                if newValue.count < current {
                    self.dispatch(.popTo(pathLen: UInt32(newValue.count)))
                }
            }
        )
    }

    func detail(for key: String) -> DetailState? {
        state.details.first { $0.key == key } ?? detailCache[key]
    }

    // MARK: Hata ayıklama / otomasyon için açılış argümanları
    //   -startScreen home|detail|benchmark   -detailId 2   -autorun YES   -query embassy
    @ObservationIgnored private var launchApplied = false

    func applyLaunchOptions() {
        guard !launchApplied else { return }
        launchApplied = true
        let d = UserDefaults.standard
        if let q = d.string(forKey: "query"), !q.isEmpty { dispatch(.setQuery(query: q)) }
        switch d.string(forKey: "startScreen") {
        case "detail":
            let id = d.integer(forKey: "detailId")
            dispatch(.openDetail(itemId: UInt32(max(id, 1))))
        case "benchmark":
            dispatch(.openBenchmark)
        default:
            break
        }
        if d.bool(forKey: "autorun") {
            dispatch(.runAllBenchmarks)
        }
    }
}

/// Rust'ın herhangi bir thread'den çağırdığı dinleyici; durumu ana thread'e taşır.
private final class Listener: StateListener, @unchecked Sendable {
    weak var bridge: CoreBridge?
    init(bridge: CoreBridge) { self.bridge = bridge }

    func onStateChanged(state: AppViewState) {
        if Thread.isMainThread {
            bridge?.apply(state)
        } else {
            DispatchQueue.main.async { [weak self] in self?.bridge?.apply(state) }
        }
    }
}
