//! # app-core
//!
//! MasterFabric Rust uygulamasının beyni. Discord'un yeni mimarisindeki gibi
//! ekranlar platformda (SwiftUI / Compose) **sadece çizilir**; uygulama durumu,
//! navigasyon, veri ve iş kuralları burada, platformdan bağımsız Rust kodunda yaşar.
//!
//! Akış: `UI olay -> Action -> App::dispatch -> AppState güncellenir -> ViewState -> UI`

pub mod action;
pub mod app;
pub mod navigation;
pub mod screens;
pub mod store;
pub mod workloads;

pub use action::Action;
pub use app::{App, AppObserver, ExternalBenchRunner, Measurement, ViewState};
pub use navigation::{NavEntry, Navigator, Route};
pub use screens::benchmark::{BenchRow, BenchStatus, BenchmarkViewState};
pub use screens::detail::DetailViewModel;
pub use screens::home::{HomeViewModel, ItemRow};
pub use store::items::{Item, ItemStore};

/// Rust çekirdeğinin nasıl derlendiğini anlatan kısa bilgi.
pub fn build_info() -> String {
    format!(
        "app-core {} · {}-{} · {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::ARCH,
        std::env::consts::OS,
        rust_profile()
    )
}

/// `release` ya da `debug`. Benchmark sonuçları yalnızca release'te anlamlıdır.
pub fn rust_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}
