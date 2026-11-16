//! `AppState` + `dispatch(Action)` loop.
//!
//! Every change goes through `dispatch`. After each change the platform receives a
//! **partial** `ViewUpdate` containing only the screen state that actually changed
//! (dirty mask), instead of a copy of the whole app state. Long work (benchmarks)
//! runs on a background thread; the UI thread is never blocked.

use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Instant;

use crate::action::Action;
use crate::ffi_bench::{self, FfiSample};
use crate::navigation::{NavEntry, Navigator, Route};
use crate::screens::benchmark::{BenchStatus, BenchmarkState, BenchmarkViewState};
use crate::screens::detail::DetailViewModel;
use crate::screens::home::HomeViewModel;
use crate::screens::info::InfoViewModel;
use crate::store::items::ItemStore;
use crate::workloads;

/// Dirty-mask bits: which parts of the view state changed.
pub mod dirty {
    pub const NAV: u8 = 1;
    pub const HOME: u8 = 1 << 1;
    pub const DETAILS: u8 = 1 << 2;
    pub const BENCH: u8 = 1 << 3;
    pub const INFO: u8 = 1 << 4;
    pub const PERF: u8 = 1 << 5;
    pub const ALL: u8 = NAV | HOME | DETAILS | BENCH | INFO | PERF;
}

/// One benchmark measurement (Rust or platform side).
#[derive(Clone, Debug, PartialEq)]
pub struct Measurement {
    pub avg_ms: f64,
    pub min_ms: f64,
    pub iterations: u32,
    pub checksum: u64,
}

/// An app-level performance metric (launch time, update latency, ...).
/// Platform-measured values are recorded via `Action::RecordPerf`; Rust adds its own.
#[derive(Clone, Debug, PartialEq)]
pub struct PerfMetric {
    pub id: String,
    pub label: String,
    pub value: f64,
    pub unit: String,
    pub detail: String,
}

/// The platform receives partial updates.
pub trait AppObserver: Send + Sync {
    fn on_update(&self, update: ViewUpdate);
}

/// Runs the same workload in the platform language (Swift/Kotlin).
/// Rust calls this from its background thread.
pub trait ExternalBenchRunner: Send + Sync {
    fn language(&self) -> String;
    fn run(&self, bench_id: &str, param: u64, iterations: u32) -> Option<Measurement>;
    /// Test #37: run one FFI-boundary variant on the platform side, which is where the
    /// platform→Rust crossing can be observed. `calls` operations per repetition,
    /// `reps` repetitions per timed sample, median of `samples` samples.
    fn run_ffi(
        &self,
        _case_id: &str,
        _variant: &str,
        _calls: u64,
        _reps: u64,
        _samples: u32,
    ) -> Option<FfiSample> {
        None
    }
}

/// Background job started by `dispatch`.
enum Job {
    Bench(Vec<String>),
    Ffi,
}

/// Full snapshot (used once at startup and in tests).
#[derive(Clone, Debug, PartialEq)]
pub struct ViewState {
    pub revision: u64,
    pub stack: Vec<NavEntry>,
    pub current: Route,
    pub home: HomeViewModel,
    /// One model per detail screen in the stack (key: `detail:<id>`).
    pub details: Vec<DetailViewModel>,
    pub benchmark: BenchmarkViewState,
    pub info: InfoViewModel,
    pub perf: Vec<PerfMetric>,
}

/// Partial update: only changed fields are `Some`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ViewUpdate {
    pub revision: u64,
    pub stack: Option<Vec<NavEntry>>,
    pub home: Option<HomeViewModel>,
    pub details: Option<Vec<DetailViewModel>>,
    pub benchmark: Option<BenchmarkViewState>,
    pub info: Option<InfoViewModel>,
    pub perf: Option<Vec<PerfMetric>>,
}

#[derive(Default)]
struct AppState {
    revision: u64,
    nav: Navigator,
    store: ItemStore,
    query: String,
    bench: BenchmarkState,
    perf: Vec<PerfMetric>,
}

impl AppState {
    fn upsert_perf(&mut self, m: PerfMetric) {
        match self.perf.iter_mut().find(|p| p.id == m.id) {
            Some(slot) => *slot = m,
            None => self.perf.push(m),
        }
    }

    fn details(&self) -> Vec<DetailViewModel> {
        let mut details: Vec<DetailViewModel> = Vec::new();
        for route in self.nav.stack() {
            if let Route::Detail { item_id } = route {
                if !details.iter().any(|d| d.item.id == *item_id) {
                    if let Some(vm) = DetailViewModel::build(&self.store, *item_id) {
                        details.push(vm);
                    }
                }
            }
        }
        details
    }
}

pub struct App {
    state: Mutex<AppState>,
    observer: Mutex<Option<Arc<dyn AppObserver>>>,
    runner: Mutex<Option<Arc<dyn ExternalBenchRunner>>>,
}

impl App {
    pub fn new() -> Arc<Self> {
        Self::with_store(ItemStore::seeded())
    }

    pub fn with_store(store: ItemStore) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(AppState {
                store,
                ..Default::default()
            }),
            observer: Mutex::new(None),
            runner: Mutex::new(None),
        })
    }

    fn lock(&self) -> MutexGuard<'_, AppState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn set_observer(&self, observer: Arc<dyn AppObserver>) {
        *self.observer.lock().unwrap_or_else(|e| e.into_inner()) = Some(observer);
    }

    pub fn set_external_runner(&self, runner: Arc<dyn ExternalBenchRunner>) {
        *self.runner.lock().unwrap_or_else(|e| e.into_inner()) = Some(runner);
        self.mutate(dirty::BENCH, |_| ());
    }

    fn external_runner(&self) -> Option<Arc<dyn ExternalBenchRunner>> {
        self.runner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Full snapshot.
    pub fn state(&self) -> ViewState {
        let attached = self.external_runner().is_some();
        let s = self.lock();
        ViewState {
            revision: s.revision,
            stack: s.nav.entries(),
            current: s.nav.current().clone(),
            home: HomeViewModel::build(&s.store, &s.query),
            details: s.details(),
            benchmark: s.bench.view(attached),
            info: InfoViewModel::build(&s.bench),
            perf: s.perf.clone(),
        }
    }

    /// Builds only the parts named by `mask`.
    fn partial(&self, mask: u8) -> ViewUpdate {
        let attached = self.external_runner().is_some();
        let s = self.lock();
        let has = |bit: u8| mask & bit != 0;
        ViewUpdate {
            revision: s.revision,
            stack: has(dirty::NAV).then(|| s.nav.entries()),
            home: has(dirty::HOME).then(|| HomeViewModel::build(&s.store, &s.query)),
            details: has(dirty::DETAILS).then(|| s.details()),
            benchmark: has(dirty::BENCH).then(|| s.bench.view(attached)),
            info: has(dirty::INFO).then(|| InfoViewModel::build(&s.bench)),
            perf: has(dirty::PERF).then(|| s.perf.clone()),
        }
    }

    /// Mutate state; notify the observer with a partial update. The lock is released
    /// before the observer runs.
    fn mutate<R>(&self, mask: u8, f: impl FnOnce(&mut AppState) -> R) -> R {
        let out = {
            let mut s = self.lock();
            let out = f(&mut s);
            if mask != 0 {
                s.revision += 1;
            }
            out
        };
        self.emit(mask);
        out
    }

    fn emit(&self, mask: u8) {
        if mask == 0 {
            return;
        }
        let observer = self
            .observer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if let Some(observer) = observer {
            observer.on_update(self.partial(mask));
        }
    }

    /// Single entry point.
    pub fn dispatch(self: &Arc<Self>, action: Action) {
        let started = Instant::now();
        let mut job = None;
        let mask = {
            let mut s = self.lock();
            let (mask, j) = reduce(&mut s, action);
            job = j.or(job);
            if mask != 0 {
                s.revision += 1;
            }
            mask
        };
        let reduce_us = started.elapsed().as_secs_f64() * 1e6;
        self.emit(mask);
        let total_us = started.elapsed().as_secs_f64() * 1e6;
        // Rust-side cost of the last UI action (reduce + partial view build + FFI callback).
        if mask & !dirty::PERF != 0 {
            let mut s = self.lock();
            s.upsert_perf(PerfMetric {
                id: "rust_dispatch".into(),
                label: "Rust dispatch (last action)".into(),
                value: total_us,
                unit: "µs".into(),
                detail: format!("reduce {reduce_us:.1} µs + partial update + callback"),
            });
        }
        if let Some(job) = job {
            let app = Arc::clone(self);
            thread::Builder::new()
                .name("mf-benchmark".into())
                .spawn(move || match job {
                    Job::Bench(ids) => app.run_benchmarks(ids),
                    Job::Ffi => app.run_ffi_bench(),
                })
                .expect("failed to start benchmark thread");
        }
    }

    /// Background thread: Rust single-thread, Rust parallel (if any), then platform (Swift).
    fn run_benchmarks(&self, ids: Vec<String>) {
        let started = Instant::now();
        let runner = self.external_runner();
        let lang = runner
            .as_ref()
            .map(|r| r.language())
            .unwrap_or_else(|| "Platform".into());
        for id in ids {
            let spec = {
                let s = self.lock();
                s.bench
                    .rows
                    .iter()
                    .find(|r| r.id == id)
                    .map(|r| (r.name.clone(), r.param, r.iterations, r.has_parallel))
            };
            let Some((name, param, iterations, has_parallel)) = spec else {
                continue;
            };

            self.mutate(dirty::BENCH, |s| {
                s.bench.current_label = Some(format!("{name} · Rust"));
                if let Some(r) = s.bench.row_mut(&id) {
                    r.status = BenchStatus::RunningRust;
                }
            });
            let rust = workloads::measure(&id, param, iterations).map(to_measurement(iterations));
            self.mutate(dirty::BENCH, |s| {
                s.bench.steps_done += 1;
                if let Some(r) = s.bench.row_mut(&id) {
                    r.rust = rust.clone();
                }
            });

            if has_parallel {
                self.mutate(dirty::BENCH, |s| {
                    s.bench.current_label = Some(format!("{name} · Rust parallel"));
                });
                let par = workloads::measure_parallel(&id, param, iterations)
                    .map(to_measurement(iterations));
                self.mutate(dirty::BENCH, |s| {
                    s.bench.steps_done += 1;
                    if let Some(r) = s.bench.row_mut(&id) {
                        r.rust_parallel = par.clone();
                    }
                });
            }

            self.mutate(dirty::BENCH, |s| {
                s.bench.current_label = Some(format!("{name} · {lang}"));
                if let Some(r) = s.bench.row_mut(&id) {
                    r.status = BenchStatus::RunningSwift;
                }
            });
            let swift = runner.as_ref().and_then(|r| r.run(&id, param, iterations));

            self.mutate(dirty::BENCH | dirty::INFO, |s| {
                s.bench.steps_done += 1;
                if let Some(r) = s.bench.row_mut(&id) {
                    r.swift = swift.clone();
                    r.status = match (&r.rust, &r.swift) {
                        (None, _) => BenchStatus::Failed {
                            message: "Rust measurement failed".into(),
                        },
                        (_, None) if runner.is_some() => BenchStatus::Failed {
                            message: format!("{lang} measurement failed"),
                        },
                        _ => BenchStatus::Done,
                    };
                    r.finalize();
                }
            });
        }
        let total = started.elapsed().as_secs_f64() * 1000.0;
        self.mutate(dirty::BENCH | dirty::INFO, |s| {
            s.bench.running = false;
            s.bench.current_label = None;
            s.bench.last_total_ms = Some(total);
        });
    }
}

impl App {
    /// Background thread for test #37. Rust decides the cases, order and parameters and
    /// interprets the results; the platform runner only times the loops it is told to.
    fn run_ffi_bench(&self) {
        let started = Instant::now();
        let runner = self.external_runner();
        let rows = self.lock().bench.ffi.rows.clone();
        for row in rows {
            let (id, case_id, label, calls, reps) = (
                row.id.clone(),
                row.case_id.clone(),
                row.label.clone(),
                row.calls,
                row.reps,
            );
            let variants = row.variants();
            for variant in variants {
                self.mutate(dirty::BENCH, |s| {
                    s.bench.ffi.current_label =
                        Some(format!("{label} · {}", variant_label(variant)));
                    if let Some(r) = s.bench.ffi.row_mut(&id) {
                        r.running = true;
                    }
                });
                let sample = runner
                    .as_ref()
                    .and_then(|r| r.run_ffi(&case_id, variant, calls, reps, ffi_bench::SAMPLES));
                self.mutate(dirty::BENCH, |s| {
                    s.bench.ffi.steps_done += 1;
                    if let Some(r) = s.bench.ffi.row_mut(&id) {
                        r.set(variant, sample.clone());
                        r.finalize();
                    }
                });
            }
            self.mutate(dirty::BENCH, |s| {
                if let Some(r) = s.bench.ffi.row_mut(&id) {
                    r.running = false;
                }
            });
        }
        let total = started.elapsed().as_secs_f64() * 1000.0;
        self.mutate(dirty::BENCH, |s| {
            s.bench.ffi.running = false;
            s.bench.ffi.current_label = None;
            s.bench.ffi.last_total_ms = Some(total);
        });
    }
}

fn variant_label(v: &str) -> &'static str {
    match v {
        ffi_bench::SWIFT_INLINE => "Swift inline",
        ffi_bench::RUST_PER_CALL => "Rust per call",
        ffi_bench::RUST_BATCHED => "Rust batched",
        _ => "?",
    }
}

fn to_measurement(iterations: u32) -> impl Fn((f64, f64, u64)) -> Measurement {
    move |(avg, min, checksum)| Measurement {
        avg_ms: avg,
        min_ms: min,
        iterations,
        checksum,
    }
}

/// Pure state transition. Returns the dirty mask and an optional benchmark job.
fn reduce(s: &mut AppState, action: Action) -> (u8, Option<Job>) {
    use dirty::*;
    match action {
        Action::OpenDetail { item_id } => {
            if s.store.get(item_id).is_some() && s.nav.current() != &(Route::Detail { item_id }) {
                s.nav.push(Route::Detail { item_id });
                return (NAV | DETAILS, None);
            }
            (0, None)
        }
        Action::OpenBenchmark => push(s, Route::Benchmark),
        Action::OpenInfo => push(s, Route::Info),
        Action::Back => {
            if s.nav.pop() {
                (NAV | DETAILS, None)
            } else {
                (0, None)
            }
        }
        Action::PopTo { path_len } => {
            let before = s.nav.stack().len();
            s.nav.pop_to(path_len as usize);
            if s.nav.stack().len() != before {
                (NAV | DETAILS, None)
            } else {
                (0, None)
            }
        }
        Action::SetQuery { query } => {
            if s.query == query {
                return (0, None);
            }
            s.query = query;
            (HOME, None)
        }
        Action::ToggleFavorite { item_id } => {
            if s.store.toggle_favorite(item_id).is_some() {
                (HOME | DETAILS, None)
            } else {
                (0, None)
            }
        }
        Action::RunBenchmark { id } => match s.bench.begin(vec![id]) {
            Some(ids) => (BENCH | INFO, Some(Job::Bench(ids))),
            None => (0, None),
        },
        Action::RunAllBenchmarks => {
            let ids = s.bench.rows.iter().map(|r| r.id.clone()).collect();
            match s.bench.begin(ids) {
                Some(ids) => (BENCH | INFO, Some(Job::Bench(ids))),
                None => (0, None),
            }
        }
        Action::RunCategory { category } => {
            let ids = s
                .bench
                .rows
                .iter()
                .filter(|r| r.category == category)
                .map(|r| r.id.clone())
                .collect();
            match s.bench.begin(ids) {
                Some(ids) => (BENCH | INFO, Some(Job::Bench(ids))),
                None => (0, None),
            }
        }
        Action::RunFfiBench => {
            if s.bench.busy() {
                return (0, None);
            }
            match s.bench.ffi.begin() {
                Some(_) => (BENCH, Some(Job::Ffi)),
                None => (0, None),
            }
        }
        Action::ResetBenchmarks => {
            s.bench.reset();
            (BENCH | INFO, None)
        }
        Action::RecordPerf {
            id,
            label,
            value,
            unit,
            detail,
        } => {
            s.upsert_perf(PerfMetric {
                id,
                label,
                value,
                unit,
                detail,
            });
            (PERF, None)
        }
    }
}

fn push(s: &mut AppState, route: Route) -> (u8, Option<Job>) {
    if s.nav.current() == &route {
        return (0, None);
    }
    s.nav.push(route);
    (dirty::NAV, None)
}
