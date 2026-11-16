//! # app-core
//!
//! The brain of the MasterFabric Rust app. "Shared Rust core, native UI":
//! screens on the platform (SwiftUI / Compose) **only render**; app state,
//! navigation, data and business rules live here in platform-independent Rust.
//!
//! Flow: `UI event -> Action -> App::dispatch -> AppState -> partial ViewUpdate -> UI`

pub mod action;
pub mod app;
pub mod heavy;
pub mod media;
pub mod navigation;
pub mod screens;
pub mod store;
pub mod workloads;

pub use action::Action;
pub use app::{
    dirty, App, AppObserver, ExternalBenchRunner, Measurement, PerfMetric, ViewState, ViewUpdate,
};
pub use navigation::{NavEntry, Navigator, Route};
pub use screens::benchmark::{BenchRow, BenchStatus, BenchmarkViewState};
pub use screens::detail::DetailViewModel;
pub use screens::home::{HomeViewModel, ItemRow};
pub use screens::info::{ArchStep, InfoViewModel, MeasuredRatio, ProjectCategory};
pub use store::items::{Item, ItemStore};

/// Short description of how the Rust core was built.
pub fn build_info() -> String {
    format!(
        "app-core {} · {}-{} · {} · {} threads",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::ARCH,
        std::env::consts::OS,
        rust_profile(),
        rayon::current_num_threads()
    )
}

/// `release` or `debug`. Benchmark results are only meaningful in release.
pub fn rust_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}
