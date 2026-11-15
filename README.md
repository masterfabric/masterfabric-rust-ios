# MasterFabric Rust (iOS)

Discord'un 2026'da duyurduğu mimarinin küçük bir örneği: **ekranlar yerel (SwiftUI) ve yalnızca çizer; uygulama durumu, navigasyon, veri ve iş kuralları paylaşılan bir Rust çekirdeğinde yaşar.** Swift ile Rust arasındaki köprüyü [UniFFI](https://mozilla.github.io/uniffi-rs/) otomatik üretir.

Uygulamada üç ekran var:

| Ekran | Ne gösterir | Rust tarafı |
|---|---|---|
| **Ana sayfa** | Rust ile yapılabilecek projelerin listesi, arama, favoriler | `screens/home.rs`, `store/items.rs` |
| **Detay** | Seçilen öğe, etiketler, favori butonu, aynı kategoriden öğeler | `screens/detail.rs` |
| **Benchmark** | 7 iş yükünü hem Rust hem Swift ile ölçer, oranları ve özet tabloyu gösterir | `screens/benchmark.rs`, `workloads.rs` |

## Mimari

```
SwiftUI ekranı ──(Action)──▶ AppCore.dispatch ──▶ app-core reducer ──▶ AppState
      ▲                                                                   │
      └──────── CoreBridge (@Observable) ◀── StateListener ◀── ViewState ─┘
```

- **Tek giriş noktası:** Swift yalnızca `core.dispatch(.openDetail(itemId: 2))` gibi olaylar gönderir.
- **Navigasyon Rust'ta:** Geri yığını `navigation.rs` içinde. SwiftUI `NavigationStack(path:)` bu yığını anahtarlarla (`detail:2`, `benchmark`) çizer. Kullanıcı kaydırarak geri dönerse Swift `popTo` gönderir.
- **İş kuralları Rust'ta:** Arama (büyük/küçük harf duyarsız, etiketlerde de arar), favorilerin üstte gösterilmesi, aynı ekranın çift dokunuşla iki kez açılmaması, benchmark sırası, oranlar, geometrik ortalama, checksum doğrulaması.
- **Arayüz olmadan test:** Sayfa akışları `cargo test` ile test edilir (`crates/app-core/tests/flows.rs`).

### Benchmark nasıl çalışır

1. `Tümünü çalıştır` → `Action::RunAllBenchmarks` Rust'a gider, Rust **arka plan thread'i** başlatır (ana thread asla bloklanmaz).
2. Her test için önce Rust iş yükü ölçülür (1 ısınma + N ölçüm).
3. Ardından Rust, Swift'in uyguladığı `SwiftBenchRunner` callback'ini aynı arka plan thread'inden çağırır; Swift aynı algoritmayı ölçer.
4. Rust oranı (`Swift ort. / Rust ort.`) ve yorumu hesaplar, iki dilin **checksum**'ını karşılaştırır, her adımda yeni durumu yayınlar.

| Test | Parametre | Not |
|---|---|---|
| Fibonacci | `fib(30)` | özyineleme / fonksiyon çağrısı |
| Sıralama | 1.000.000 × u32 | xorshift ile üret + sırala |
| SHA-256 | 1 MiB | iki dilde elle yazılmış (CryptoKit kullanılmadı: donanım hızlandırmalı olduğu için karşılaştırmayı bozar) |
| JSON | 5.000 kayıt | `serde_json` vs `Codable` (yaz + oku) |
| Asal elek | n = 5.000.000 | Eratosthenes |
| Matris | 200 × 200 f64 | i-k-j döngüsü |
| HashMap | 300.000 ekleme | `HashMap` vs `Dictionary` |

Parametreler Rust'tan çalışma anında gelir; böylece derleyiciler sonucu derleme anında hesaplayamaz. İki taraf da idiomatic kod (unsafe yok).

> **Uyarı:** Simülatörde ölçülen işlemci Mac'in CPU'sudur, gerçek iPhone değil. Swift tarafı Debug derlenirse sonuçlar anlamsız derecede yavaş çıkar; ekran bunu uyarı olarak gösterir. Anlamlı sonuç için Rust her zaman `--release`, Xcode `Release` ile derlenir.

## Klasör yapısı

```
masterfabric-rust-ios/
├── Cargo.toml                    # Rust workspace (release: LTO, codegen-units=1)
├── crates/
│   ├── app-core/                 # Uygulamanın beyni (platformdan bağımsız)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── app.rs            # AppState + dispatch(Action), arka plan benchmark thread'i
│   │   │   ├── action.rs         # Kullanıcı olayları
│   │   │   ├── navigation.rs     # Route + geri yığını
│   │   │   ├── screens/{home,detail,benchmark}.rs   # View-model'ler
│   │   │   ├── store/items.rs    # Veri katmanı
│   │   │   └── workloads.rs      # Benchmark iş yükleri (Rust)
│   │   └── tests/flows.rs        # Akış testleri (UI olmadan)
│   ├── app-ffi/                  # UniFFI köprüsü: AppCore, StateListener, SwiftBenchRunner
│   └── uniffi-bindgen/           # Swift bağlamalarını üreten küçük CLI
├── platforms/ios/
│   ├── project.yml               # XcodeGen tanımı (.xcodeproj buradan üretilir)
│   ├── Frameworks/AppCore.xcframework    # (üretilir) derlenmiş Rust
│   └── MasterFabricRust/
│       ├── MasterFabricRustApp.swift
│       ├── Bridge/CoreBridge.swift       # Rust durumunun @Observable aynası
│       ├── Bridge/DeviceInfo.swift
│       ├── Benchmarks/SwiftWorkloads.swift   # Aynı iş yüklerinin Swift hali
│       ├── Benchmarks/SwiftRunner.swift      # Rust'ın çağırdığı Swift ölçücü
│       ├── Screens/{RootView,HomeScreen,DetailScreen,BenchmarkScreen}.swift
│       └── Generated/                    # (üretilir) UniFFI Swift kodu
└── scripts/
    ├── build-ios.sh              # Rust → iOS + simülatör, UniFFI, XCFramework, XcodeGen
    └── run-sim.sh                # Derle, açık simülatöre kur ve başlat
```

## Kurulum ve çalıştırma

Gerekenler: Xcode, [rustup](https://rustup.rs), `brew install xcodegen`.

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim

cargo test                      # çekirdek + akış testleri
cargo clippy --all-targets

./scripts/build-ios.sh          # Rust'ı derler, Swift köprüsünü ve xcodeproj'u üretir
./scripts/run-sim.sh            # Release derler, simülatöre kurar, açar
open platforms/ios/MasterFabricRust.xcodeproj   # ya da Xcode'dan çalıştırın
```

Rust kodunu değiştirdikten sonra `./scripts/build-ios.sh` komutunu tekrar çalıştırın.

### Hata ayıklama / otomasyon argümanları

```bash
./scripts/run-sim.sh -startScreen detail -detailId 2
./scripts/run-sim.sh -startScreen benchmark -autorun YES
./scripts/run-sim.sh -startScreen benchmark -autorun YES -scrollTo sha256
./scripts/run-sim.sh -query embassy
```

Bu argümanlar da yalnızca Rust'a `Action` gönderir; navigasyon yine Rust'ta kalır.

## Sonraki adımlar

- `platforms/android/` altında aynı çekirdeği Jetpack Compose ile çizmek (UniFFI Kotlin bağlamaları).
- `storage.rs` ile favorileri SQLite'ta kalıcı tutmak.
- Gerçek iPhone'da benchmark (Signing ayarı gerekir).
