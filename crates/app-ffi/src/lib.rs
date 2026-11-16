//! # app-ffi
//!
//! Thin bridge exposing `app-core` to Swift (and later Kotlin) via UniFFI.
//! No business logic here: only type conversions and callback adapters.
//!
//! FFI cost notes: state changes are sent as *partial* `ViewUpdate`s (only changed
//! screens), large pixel buffers cross once as `Vec<u8>` (bytes → `Data`), and
//! there are no per-item calls.

use std::sync::Arc;

use app_core as core;

uniffi::setup_scaffolding!();

// ------------------------------------------------------------------ types

#[derive(uniffi::Enum, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Route {
    Home,
    Detail { item_id: u32 },
    Benchmark,
    Info,
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
    pub category: String,
    pub has_parallel: bool,
    pub status: BenchStatus,
    pub rust: Option<BenchMeasurement>,
    pub rust_parallel: Option<BenchMeasurement>,
    pub swift: Option<BenchMeasurement>,
    pub ratio: Option<f64>,
    pub parallel_speedup: Option<f64>,
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
    pub rust_threads: u32,
    pub swift_runner_attached: bool,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct PerfMetric {
    pub id: String,
    pub label: String,
    pub value: f64,
    pub unit: String,
    pub detail: String,
}

/// Partial update: only changed screen state is `Some`.
#[derive(uniffi::Record, Clone, Debug)]
pub struct ViewUpdate {
    pub revision: u64,
    pub stack: Option<Vec<NavEntry>>,
    pub home: Option<HomeState>,
    pub details: Option<Vec<DetailState>>,
    pub benchmark: Option<BenchmarkState>,
    pub info: Option<InfoState>,
    pub perf: Option<Vec<PerfMetric>>,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct AppViewState {
    pub revision: u64,
    pub stack: Vec<NavEntry>,
    pub current: Route,
    pub home: HomeState,
    pub details: Vec<DetailState>,
    pub benchmark: BenchmarkState,
    pub info: InfoState,
    pub perf: Vec<PerfMetric>,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct ProjectCategory {
    pub id: String,
    pub title: String,
    pub symbol: String,
    pub tint: String,
    pub summary: String,
    pub examples: Vec<String>,
    pub crates: Vec<String>,
    pub related_bench_ids: Vec<String>,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct ArchStep {
    pub title: String,
    pub detail: String,
    pub symbol: String,
    pub layer: String,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct MeasuredRatio {
    pub bench_id: String,
    pub name: String,
    pub category: String,
    pub rust_ms: f64,
    pub swift_ms: f64,
    pub ratio: f64,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct InfoState {
    pub title: String,
    pub intro: String,
    pub architecture: Vec<ArchStep>,
    pub categories: Vec<ProjectCategory>,
    pub measured: Vec<MeasuredRatio>,
    pub measured_note: String,
    pub strengths: Vec<String>,
    pub tradeoffs: Vec<String>,
}

/// Summary of a 3D model generated in Rust.
#[derive(uniffi::Record, Clone, Debug)]
pub struct MeshSummary {
    pub vertices: u32,
    pub triangles: u32,
    pub obj_bytes: u32,
    pub checksum: u64,
}

#[derive(uniffi::Enum, Clone, Debug)]
pub enum Action {
    OpenDetail {
        item_id: u32,
    },
    OpenBenchmark,
    OpenInfo,
    Back,
    PopTo {
        path_len: u32,
    },
    SetQuery {
        query: String,
    },
    ToggleFavorite {
        item_id: u32,
    },
    RunBenchmark {
        id: String,
    },
    RunAllBenchmarks,
    RunCategory {
        category: String,
    },
    ResetBenchmarks,
    RecordPerf {
        id: String,
        label: String,
        value: f64,
        unit: String,
        detail: String,
    },
}

// ------------------------------------------------------------------ callbacks

/// Implemented by Swift; called (from any thread) with a partial update after each change.
#[uniffi::export(with_foreign)]
pub trait StateListener: Send + Sync {
    fn on_update(&self, update: ViewUpdate);
}

/// Implemented by Swift; measures the same workload in Swift. Called from Rust's background thread.
#[uniffi::export(with_foreign)]
pub trait SwiftBenchRunner: Send + Sync {
    fn language(&self) -> String;
    fn run(&self, bench_id: String, param: u64, iterations: u32) -> Option<BenchMeasurement>;
}

struct ListenerAdapter(Arc<dyn StateListener>);
impl core::AppObserver for ListenerAdapter {
    fn on_update(&self, u: core::ViewUpdate) {
        self.0.on_update(ViewUpdate {
            revision: u.revision,
            stack: u.stack.map(|v| v.into_iter().map(Into::into).collect()),
            home: u.home.map(Into::into),
            details: u.details.map(|v| v.into_iter().map(Into::into).collect()),
            benchmark: u.benchmark.map(Into::into),
            info: u.info.map(Into::into),
            perf: u.perf.map(|v| v.into_iter().map(Into::into).collect()),
        });
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

// ------------------------------------------------------------------ object

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

    /// Single entry point: send a user event to Rust.
    pub fn dispatch(&self, action: Action) {
        self.app.dispatch(action.into());
    }

    /// Full snapshot (startup only; updates arrive as partial `ViewUpdate`s).
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

    /// RGBA8 output of the Rust ray tracer (same code as the benchmark), as visual proof.
    /// One call, one buffer (`Vec<u8>` → `Data`). Rendered with rayon. Call off the main thread.
    pub fn raytrace_image(&self, width: u32, height: u32) -> Vec<u8> {
        core::media::raytrace_par(width as usize, height as usize)
    }

    /// Generates a procedural 3D model in Rust (same code as the benchmark).
    pub fn generate_mesh(&self, levels: u32) -> MeshSummary {
        let (v, f, obj, checksum) = core::media::generate_mesh(levels.min(7));
        MeshSummary {
            vertices: v as u32,
            triangles: f as u32,
            obj_bytes: obj as u32,
            checksum,
        }
    }
}

// ------------------------------------------------------------------ conversions

impl From<core::Route> for Route {
    fn from(r: core::Route) -> Self {
        match r {
            core::Route::Home => Route::Home,
            core::Route::Detail { item_id } => Route::Detail { item_id },
            core::Route::Benchmark => Route::Benchmark,
            core::Route::Info => Route::Info,
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
            category: r.category,
            has_parallel: r.has_parallel,
            status: r.status.into(),
            rust: r.rust.map(Into::into),
            rust_parallel: r.rust_parallel.map(Into::into),
            swift: r.swift.map(Into::into),
            ratio: r.ratio,
            parallel_speedup: r.parallel_speedup,
            verdict: r.verdict,
            checksum_match: r.checksum_match,
        }
    }
}

impl From<core::NavEntry> for NavEntry {
    fn from(e: core::NavEntry) -> Self {
        Self {
            key: e.key,
            route: e.route.into(),
        }
    }
}

impl From<core::HomeViewModel> for HomeState {
    fn from(h: core::HomeViewModel) -> Self {
        Self {
            title: h.title,
            query: h.query,
            rows: h.rows.into_iter().map(Into::into).collect(),
            total_count: h.total_count,
            favorite_count: h.favorite_count,
            empty_message: h.empty_message,
        }
    }
}

impl From<core::DetailViewModel> for DetailState {
    fn from(d: core::DetailViewModel) -> Self {
        Self {
            key: d.key,
            item: d.item.into(),
            favorite_label: d.favorite_label,
            related: d.related.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<core::BenchmarkViewState> for BenchmarkState {
    fn from(b: core::BenchmarkViewState) -> Self {
        Self {
            rows: b.rows.into_iter().map(Into::into).collect(),
            running: b.running,
            progress: b.progress,
            current_label: b.current_label,
            last_total_ms: b.last_total_ms,
            summary: b.summary,
            rust_profile: b.rust_profile,
            core_info: b.core_info,
            rust_threads: b.rust_threads,
            swift_runner_attached: b.swift_runner_attached,
        }
    }
}

impl From<core::InfoViewModel> for InfoState {
    fn from(i: core::InfoViewModel) -> Self {
        Self {
            title: i.title,
            intro: i.intro,
            architecture: i
                .architecture
                .into_iter()
                .map(|a| ArchStep {
                    title: a.title,
                    detail: a.detail,
                    symbol: a.symbol,
                    layer: a.layer,
                })
                .collect(),
            categories: i
                .categories
                .into_iter()
                .map(|c| ProjectCategory {
                    id: c.id,
                    title: c.title,
                    symbol: c.symbol,
                    tint: c.tint,
                    summary: c.summary,
                    examples: c.examples,
                    crates: c.crates,
                    related_bench_ids: c.related_bench_ids,
                })
                .collect(),
            measured: i
                .measured
                .into_iter()
                .map(|m| MeasuredRatio {
                    bench_id: m.bench_id,
                    name: m.name,
                    category: m.category,
                    rust_ms: m.rust_ms,
                    swift_ms: m.swift_ms,
                    ratio: m.ratio,
                })
                .collect(),
            measured_note: i.measured_note,
            strengths: i.strengths,
            tradeoffs: i.tradeoffs,
        }
    }
}

impl From<core::PerfMetric> for PerfMetric {
    fn from(m: core::PerfMetric) -> Self {
        Self {
            id: m.id,
            label: m.label,
            value: m.value,
            unit: m.unit,
            detail: m.detail,
        }
    }
}

impl From<core::ViewState> for AppViewState {
    fn from(s: core::ViewState) -> Self {
        Self {
            revision: s.revision,
            stack: s.stack.into_iter().map(Into::into).collect(),
            current: s.current.into(),
            home: s.home.into(),
            details: s.details.into_iter().map(Into::into).collect(),
            benchmark: s.benchmark.into(),
            info: s.info.into(),
            perf: s.perf.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<Action> for core::Action {
    fn from(a: Action) -> Self {
        match a {
            Action::OpenDetail { item_id } => core::Action::OpenDetail { item_id },
            Action::OpenBenchmark => core::Action::OpenBenchmark,
            Action::OpenInfo => core::Action::OpenInfo,
            Action::Back => core::Action::Back,
            Action::PopTo { path_len } => core::Action::PopTo { path_len },
            Action::SetQuery { query } => core::Action::SetQuery { query },
            Action::ToggleFavorite { item_id } => core::Action::ToggleFavorite { item_id },
            Action::RunBenchmark { id } => core::Action::RunBenchmark { id },
            Action::RunAllBenchmarks => core::Action::RunAllBenchmarks,
            Action::RunCategory { category } => core::Action::RunCategory { category },
            Action::ResetBenchmarks => core::Action::ResetBenchmarks,
            Action::RecordPerf {
                id,
                label,
                value,
                unit,
                detail,
            } => core::Action::RecordPerf {
                id,
                label,
                value,
                unit,
                detail,
            },
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
        assert!(s.benchmark.rows.len() >= 30);
        assert_eq!(s.benchmark.rows[0].category, "Image");
        assert!(!s.info.categories.is_empty());
    }
}
