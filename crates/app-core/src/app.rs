//! `AppState` + `dispatch(Action)` döngüsü.
//!
//! Tüm değişiklikler `dispatch` üzerinden geçer. Her değişiklikten sonra
//! platforma tek bir `ViewState` anlık görüntüsü gönderilir; platform yalnızca çizer.
//! Uzun işler (benchmark) arka plan iş parçacığında çalışır, ana thread asla bloklanmaz.

use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Instant;

use crate::action::Action;
use crate::navigation::{NavEntry, Navigator, Route};
use crate::screens::benchmark::{BenchStatus, BenchmarkState, BenchmarkViewState};
use crate::screens::detail::DetailViewModel;
use crate::screens::home::HomeViewModel;
use crate::store::items::ItemStore;
use crate::workloads;

/// Bir benchmark ölçümünün sonucu (Rust ya da platform tarafı).
#[derive(Clone, Debug, PartialEq)]
pub struct Measurement {
    pub avg_ms: f64,
    pub min_ms: f64,
    pub iterations: u32,
    pub checksum: u64,
}

/// Platform her durum değişikliğinde bunu alır.
pub trait AppObserver: Send + Sync {
    fn on_state_changed(&self, state: ViewState);
}

/// Aynı iş yükünü platformun kendi dilinde (Swift/Kotlin) çalıştıran nesne.
/// Rust bunu arka plan thread'inden çağırır.
pub trait ExternalBenchRunner: Send + Sync {
    fn language(&self) -> String;
    fn run(&self, bench_id: &str, param: u64, iterations: u32) -> Option<Measurement>;
}

/// Platformun çizdiği her şey.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewState {
    pub revision: u64,
    pub stack: Vec<NavEntry>,
    pub current: Route,
    pub home: HomeViewModel,
    /// Yığındaki her detay ekranı için bir model (anahtar: `detail:<id>`).
    pub details: Vec<DetailViewModel>,
    pub benchmark: BenchmarkViewState,
}

#[derive(Default)]
struct AppState {
    revision: u64,
    nav: Navigator,
    store: ItemStore,
    query: String,
    bench: BenchmarkState,
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
        self.emit();
    }

    pub fn set_external_runner(&self, runner: Arc<dyn ExternalBenchRunner>) {
        *self.runner.lock().unwrap_or_else(|e| e.into_inner()) = Some(runner);
        self.emit();
    }

    fn external_runner(&self) -> Option<Arc<dyn ExternalBenchRunner>> {
        self.runner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Anlık görüntü.
    pub fn state(&self) -> ViewState {
        let attached = self.external_runner().is_some();
        let s = self.lock();
        Self::snapshot(&s, attached)
    }

    fn snapshot(s: &AppState, runner_attached: bool) -> ViewState {
        let mut details: Vec<DetailViewModel> = Vec::new();
        for route in s.nav.stack() {
            if let Route::Detail { item_id } = route {
                if !details.iter().any(|d| d.item.id == *item_id) {
                    if let Some(vm) = DetailViewModel::build(&s.store, *item_id) {
                        details.push(vm);
                    }
                }
            }
        }
        ViewState {
            revision: s.revision,
            stack: s.nav.entries(),
            current: s.nav.current().clone(),
            home: HomeViewModel::build(&s.store, &s.query),
            details,
            benchmark: s.bench.view(runner_attached),
        }
    }

    /// Durumu değiştir, gözlemciyi bilgilendir. Kilit gözlemci çağrılmadan bırakılır.
    fn mutate<R>(&self, f: impl FnOnce(&mut AppState) -> R) -> R {
        let out = {
            let mut s = self.lock();
            let out = f(&mut s);
            s.revision += 1;
            out
        };
        self.emit();
        out
    }

    fn emit(&self) {
        let observer = self
            .observer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if let Some(observer) = observer {
            observer.on_state_changed(self.state());
        }
    }

    /// Tek giriş noktası.
    pub fn dispatch(self: &Arc<Self>, action: Action) {
        let job = self.mutate(|s| reduce(s, action));
        if let Some(ids) = job {
            let app = Arc::clone(self);
            thread::Builder::new()
                .name("mf-benchmark".into())
                .spawn(move || app.run_benchmarks(ids))
                .expect("benchmark thread başlatılamadı");
        }
    }

    /// Arka plan thread'i: önce Rust, sonra platform (Swift) ölçümü.
    fn run_benchmarks(&self, ids: Vec<String>) {
        let started = Instant::now();
        let runner = self.external_runner();
        for id in ids {
            let spec = {
                let s = self.lock();
                s.bench
                    .rows
                    .iter()
                    .find(|r| r.id == id)
                    .map(|r| (r.name.clone(), r.param, r.iterations))
            };
            let Some((name, param, iterations)) = spec else {
                continue;
            };

            self.mutate(|s| {
                s.bench.current_label = Some(format!("{name} · Rust"));
                if let Some(r) = s.bench.row_mut(&id) {
                    r.status = BenchStatus::RunningRust;
                }
            });
            let rust = workloads::measure(&id, param, iterations).map(|(avg, min, checksum)| {
                Measurement {
                    avg_ms: avg,
                    min_ms: min,
                    iterations,
                    checksum,
                }
            });

            let lang = runner
                .as_ref()
                .map(|r| r.language())
                .unwrap_or_else(|| "Platform".into());
            self.mutate(|s| {
                s.bench.steps_done += 1;
                s.bench.current_label = Some(format!("{name} · {lang}"));
                if let Some(r) = s.bench.row_mut(&id) {
                    r.rust = rust.clone();
                    r.status = BenchStatus::RunningSwift;
                }
            });

            let swift = runner.as_ref().and_then(|r| r.run(&id, param, iterations));

            self.mutate(|s| {
                s.bench.steps_done += 1;
                if let Some(r) = s.bench.row_mut(&id) {
                    r.swift = swift.clone();
                    r.status = match (&r.rust, &r.swift) {
                        (None, _) => BenchStatus::Failed {
                            message: "Rust ölçümü başarısız".into(),
                        },
                        (_, None) if runner.is_some() => BenchStatus::Failed {
                            message: format!("{lang} ölçümü başarısız"),
                        },
                        _ => BenchStatus::Done,
                    };
                    r.finalize();
                }
            });
        }
        let total = started.elapsed().as_secs_f64() * 1000.0;
        self.mutate(|s| {
            s.bench.running = false;
            s.bench.current_label = None;
            s.bench.last_total_ms = Some(total);
        });
    }
}

/// Saf durum geçişi. Arka planda çalıştırılacak benchmark listesi döndürebilir.
fn reduce(s: &mut AppState, action: Action) -> Option<Vec<String>> {
    match action {
        Action::OpenDetail { item_id } => {
            if s.store.get(item_id).is_some() {
                s.nav.push(Route::Detail { item_id });
            }
        }
        Action::OpenBenchmark => s.nav.push(Route::Benchmark),
        Action::Back => {
            s.nav.pop();
        }
        Action::PopTo { path_len } => s.nav.pop_to(path_len as usize),
        Action::SetQuery { query } => s.query = query,
        Action::ToggleFavorite { item_id } => {
            s.store.toggle_favorite(item_id);
        }
        Action::RunBenchmark { id } => return s.bench.begin(vec![id]),
        Action::RunAllBenchmarks => {
            let ids = s.bench.rows.iter().map(|r| r.id.clone()).collect();
            return s.bench.begin(ids);
        }
        Action::ResetBenchmarks => s.bench.reset(),
    }
    None
}
