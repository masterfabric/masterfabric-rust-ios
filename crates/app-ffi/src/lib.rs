//! # app-ffi
//!
//! `app-core`'u UniFFI ile Swift'e (ve ileride Kotlin'e) açan ince köprü.
//! Burada iş mantığı YOKTUR: yalnızca tip dönüşümleri ve callback adaptörleri.

use std::sync::Arc;

use app_core as core;

uniffi::setup_scaffolding!();

// ------------------------------------------------------------------ tipler

#[derive(uniffi::Enum, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Route {
    Home,
    Detail { item_id: u32 },
    Benchmark,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct NavEntry {
    pub key: String,
    pub route: Route,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct ItemRow {
    pub id: u32,
    pub title: String,
    pub subtitle: String,
    pub category: String,
    pub symbol: String,
    pub favorite: bool,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct Item {
    pub id: u32,
    pub title: String,
    pub subtitle: String,
    pub body: String,
    pub category: String,
    pub symbol: String,
    pub tags: Vec<String>,
    pub favorite: bool,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct HomeState {
    pub title: String,
    pub query: String,
    pub rows: Vec<ItemRow>,
    pub total_count: u32,
    pub favorite_count: u32,
    pub empty_message: Option<String>,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct DetailState {
    pub key: String,
    pub item: Item,
    pub favorite_label: String,
    pub related: Vec<ItemRow>,
}

#[derive(uniffi::Enum, Clone, Debug, PartialEq)]
pub enum BenchStatus {
    Idle,
    Queued,
    RunningRust,
    RunningSwift,
    Done,
    Failed { message: String },
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct BenchMeasurement {
    pub avg_ms: f64,
    pub min_ms: f64,
    pub iterations: u32,
    pub checksum: u64,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct BenchRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub symbol: String,
    pub param: u64,
    pub param_label: String,
    pub iterations: u32,
    pub status: BenchStatus,
    pub rust: Option<BenchMeasurement>,
    pub swift: Option<BenchMeasurement>,
    pub ratio: Option<f64>,
    pub verdict: Option<String>,
    pub checksum_match: Option<bool>,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct BenchmarkState {
    pub rows: Vec<BenchRow>,
    pub running: bool,
    pub progress: f64,
    pub current_label: Option<String>,
    pub last_total_ms: Option<f64>,
    pub summary: Option<String>,
    pub rust_profile: String,
    pub core_info: String,
    pub swift_runner_attached: bool,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct AppViewState {
    pub revision: u64,
    pub stack: Vec<NavEntry>,
    pub current: Route,
    pub home: HomeState,
    pub details: Vec<DetailState>,
    pub benchmark: BenchmarkState,
}

#[derive(uniffi::Enum, Clone, Debug)]
pub enum Action {
    OpenDetail { item_id: u32 },
    OpenBenchmark,
    Back,
    PopTo { path_len: u32 },
    SetQuery { query: String },
    ToggleFavorite { item_id: u32 },
    RunBenchmark { id: String },
    RunAllBenchmarks,
    ResetBenchmarks,
}

// ------------------------------------------------------------------ callback'ler

/// Swift tarafı uygular; her durum değişikliğinde (herhangi bir thread'den) çağrılır.
#[uniffi::export(with_foreign)]
pub trait StateListener: Send + Sync {
    fn on_state_changed(&self, state: AppViewState);
}

/// Swift tarafı uygular; aynı iş yükünü Swift ile ölçer. Rust'ın arka plan thread'inden çağrılır.
#[uniffi::export(with_foreign)]
pub trait SwiftBenchRunner: Send + Sync {
    fn language(&self) -> String;
    fn run(&self, bench_id: String, param: u64, iterations: u32) -> Option<BenchMeasurement>;
}

struct ListenerAdapter(Arc<dyn StateListener>);
impl core::AppObserver for ListenerAdapter {
    fn on_state_changed(&self, state: core::ViewState) {
        self.0.on_state_changed(state.into());
    }
}

struct RunnerAdapter(Arc<dyn SwiftBenchRunner>);
impl core::ExternalBenchRunner for RunnerAdapter {
    fn language(&self) -> String {
        self.0.language()
    }
    fn run(&self, bench_id: &str, param: u64, iterations: u32) -> Option<core::Measurement> {
        self.0
            .run(bench_id.to_string(), param, iterations)
            .map(|m| core::Measurement {
                avg_ms: m.avg_ms,
                min_ms: m.min_ms,
                iterations: m.iterations,
                checksum: m.checksum,
            })
    }
}

// ------------------------------------------------------------------ nesne

#[derive(uniffi::Object)]
pub struct AppCore {
    app: Arc<core::App>,
}

impl Default for AppCore {
    fn default() -> Self {
        Self {
            app: core::App::new(),
        }
    }
}

#[uniffi::export]
impl AppCore {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Tek giriş noktası: kullanıcı olayını Rust'a gönder.
    pub fn dispatch(&self, action: Action) {
        self.app.dispatch(action.into());
    }

    /// Anlık durum.
    pub fn state(&self) -> AppViewState {
        self.app.state().into()
    }

    pub fn set_listener(&self, listener: Arc<dyn StateListener>) {
        self.app.set_observer(Arc::new(ListenerAdapter(listener)));
    }

    pub fn set_swift_runner(&self, runner: Arc<dyn SwiftBenchRunner>) {
        self.app
            .set_external_runner(Arc::new(RunnerAdapter(runner)));
    }

    pub fn core_info(&self) -> String {
        core::build_info()
    }
}

// ------------------------------------------------------------------ dönüşümler

impl From<core::Route> for Route {
    fn from(r: core::Route) -> Self {
        match r {
            core::Route::Home => Route::Home,
            core::Route::Detail { item_id } => Route::Detail { item_id },
            core::Route::Benchmark => Route::Benchmark,
        }
    }
}

impl From<core::ItemRow> for ItemRow {
    fn from(r: core::ItemRow) -> Self {
        Self {
            id: r.id,
            title: r.title,
            subtitle: r.subtitle,
            category: r.category,
            symbol: r.symbol,
            favorite: r.favorite,
        }
    }
}

impl From<core::Item> for Item {
    fn from(i: core::Item) -> Self {
        Self {
            id: i.id,
            title: i.title,
            subtitle: i.subtitle,
            body: i.body,
            category: i.category,
            symbol: i.symbol,
            tags: i.tags,
            favorite: i.favorite,
        }
    }
}

impl From<core::Measurement> for BenchMeasurement {
    fn from(m: core::Measurement) -> Self {
        Self {
            avg_ms: m.avg_ms,
            min_ms: m.min_ms,
            iterations: m.iterations,
            checksum: m.checksum,
        }
    }
}

impl From<core::BenchStatus> for BenchStatus {
    fn from(s: core::BenchStatus) -> Self {
        match s {
            core::BenchStatus::Idle => BenchStatus::Idle,
            core::BenchStatus::Queued => BenchStatus::Queued,
            core::BenchStatus::RunningRust => BenchStatus::RunningRust,
            core::BenchStatus::RunningSwift => BenchStatus::RunningSwift,
            core::BenchStatus::Done => BenchStatus::Done,
            core::BenchStatus::Failed { message } => BenchStatus::Failed { message },
        }
    }
}

impl From<core::BenchRow> for BenchRow {
    fn from(r: core::BenchRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            description: r.description,
            symbol: r.symbol,
            param: r.param,
            param_label: r.param_label,
            iterations: r.iterations,
            status: r.status.into(),
            rust: r.rust.map(Into::into),
            swift: r.swift.map(Into::into),
            ratio: r.ratio,
            verdict: r.verdict,
            checksum_match: r.checksum_match,
        }
    }
}

impl From<core::ViewState> for AppViewState {
    fn from(s: core::ViewState) -> Self {
        let b = s.benchmark;
        Self {
            revision: s.revision,
            stack: s
                .stack
                .into_iter()
                .map(|e| NavEntry {
                    key: e.key,
                    route: e.route.into(),
                })
                .collect(),
            current: s.current.into(),
            home: HomeState {
                title: s.home.title,
                query: s.home.query,
                rows: s.home.rows.into_iter().map(Into::into).collect(),
                total_count: s.home.total_count,
                favorite_count: s.home.favorite_count,
                empty_message: s.home.empty_message,
            },
            details: s
                .details
                .into_iter()
                .map(|d| DetailState {
                    key: d.key,
                    item: d.item.into(),
                    favorite_label: d.favorite_label,
                    related: d.related.into_iter().map(Into::into).collect(),
                })
                .collect(),
            benchmark: BenchmarkState {
                rows: b.rows.into_iter().map(Into::into).collect(),
                running: b.running,
                progress: b.progress,
                current_label: b.current_label,
                last_total_ms: b.last_total_ms,
                summary: b.summary,
                rust_profile: b.rust_profile,
                core_info: b.core_info,
                swift_runner_attached: b.swift_runner_attached,
            },
        }
    }
}

impl From<Action> for core::Action {
    fn from(a: Action) -> Self {
        match a {
            Action::OpenDetail { item_id } => core::Action::OpenDetail { item_id },
            Action::OpenBenchmark => core::Action::OpenBenchmark,
            Action::Back => core::Action::Back,
            Action::PopTo { path_len } => core::Action::PopTo { path_len },
            Action::SetQuery { query } => core::Action::SetQuery { query },
            Action::ToggleFavorite { item_id } => core::Action::ToggleFavorite { item_id },
            Action::RunBenchmark { id } => core::Action::RunBenchmark { id },
            Action::RunAllBenchmarks => core::Action::RunAllBenchmarks,
            Action::ResetBenchmarks => core::Action::ResetBenchmarks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_dispatch_roundtrip() {
        let core = AppCore::new();
        core.dispatch(Action::OpenDetail { item_id: 3 });
        let s = core.state();
        assert_eq!(s.current, Route::Detail { item_id: 3 });
        assert_eq!(s.details.len(), 1);
        assert_eq!(s.benchmark.rows.len(), 7);
    }
}
