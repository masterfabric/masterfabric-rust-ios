//! User events. The platform layer only sends these.

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// An item on Home was tapped.
    OpenDetail { item_id: u32 },
    /// Open the benchmark screen.
    OpenBenchmark,
    /// Open the "What can you build with Rust?" info screen.
    OpenInfo,
    /// Go back one screen.
    Back,
    /// Sync when the platform navigation stack shrinks (e.g. swipe back).
    /// `path_len`: number of screens after the root (Home).
    PopTo { path_len: u32 },
    /// Home search text.
    SetQuery { query: String },
    /// Toggle favorite (business rule: favorites are listed first).
    ToggleFavorite { item_id: u32 },
    /// Run one benchmark.
    RunBenchmark { id: String },
    /// Run all benchmarks in order.
    RunAllBenchmarks,
    /// Run every benchmark of one category (e.g. "Image").
    RunCategory { category: String },
    /// Run test #37: the FFI boundary / chatty-calls section.
    RunFfiBench,
    /// Clear benchmark results.
    ResetBenchmarks,
    /// Record a platform-measured performance metric (launch time, latency, ...).
    RecordPerf {
        id: String,
        label: String,
        value: f64,
        unit: String,
        detail: String,
    },
}
