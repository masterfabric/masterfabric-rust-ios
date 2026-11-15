//! Tests screen flows without any UI (a big win of the shared-core architecture).

use app_core::{
    Action, App, AppObserver, BenchStatus, ExternalBenchRunner, Measurement, Route, ViewUpdate,
};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Default)]
struct Recorder(Mutex<Vec<ViewUpdate>>);
impl AppObserver for Recorder {
    fn on_update(&self, update: ViewUpdate) {
        self.0.lock().unwrap().push(update);
    }
}

fn wait_idle(app: &Arc<App>) {
    let deadline = Instant::now() + Duration::from_secs(120);
    while app.state().benchmark.running {
        assert!(Instant::now() < deadline, "benchmark timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn home_to_detail_and_back() {
    let app = App::new();
    assert_eq!(app.state().current, Route::Home);
    assert_eq!(app.state().home.rows.len(), 8);

    app.dispatch(Action::OpenDetail { item_id: 2 });
    let s = app.state();
    assert_eq!(s.current, Route::Detail { item_id: 2 });
    assert_eq!(s.stack.len(), 2);
    assert_eq!(s.stack[1].key, "detail:2");
    assert_eq!(s.details[0].item.title, "ESP32-C6 Sensor Node");

    // A double tap does not open the same screen twice.
    app.dispatch(Action::OpenDetail { item_id: 2 });
    assert_eq!(app.state().stack.len(), 2);

    app.dispatch(Action::Back);
    assert_eq!(app.state().current, Route::Home);
    // Cannot go back from the root.
    app.dispatch(Action::Back);
    assert_eq!(app.state().stack.len(), 1);
}

#[test]
fn unknown_item_does_not_navigate() {
    let app = App::new();
    app.dispatch(Action::OpenDetail { item_id: 999 });
    assert_eq!(app.state().current, Route::Home);
}

#[test]
fn swipe_back_syncs_with_pop_to() {
    let app = App::new();
    app.dispatch(Action::OpenDetail { item_id: 1 });
    app.dispatch(Action::OpenDetail { item_id: 7 });
    app.dispatch(Action::OpenBenchmark);
    assert_eq!(app.state().stack.len(), 4);
    assert_eq!(app.state().details.len(), 2);
    app.dispatch(Action::PopTo { path_len: 1 });
    let s = app.state();
    assert_eq!(s.current, Route::Detail { item_id: 1 });
    assert_eq!(s.details.len(), 1);
}

#[test]
fn search_and_favorites_business_rules() {
    let app = App::new();
    app.dispatch(Action::SetQuery {
        query: "  EMBASSY ".into(),
    });
    let ids: Vec<u32> = app.state().home.rows.iter().map(|r| r.id).collect();
    assert_eq!(ids, vec![2, 4]);

    app.dispatch(Action::SetQuery {
        query: "no-such-thing".into(),
    });
    assert!(app.state().home.rows.is_empty());
    assert!(app.state().home.empty_message.is_some());

    app.dispatch(Action::SetQuery {
        query: String::new(),
    });
    app.dispatch(Action::ToggleFavorite { item_id: 5 });
    let s = app.state();
    assert_eq!(s.home.rows[0].id, 5, "favorites first");
    assert_eq!(s.home.favorite_count, 1);
}

#[test]
fn observer_receives_partial_updates() {
    let app = App::new();
    let rec = Arc::new(Recorder::default());
    app.set_observer(rec.clone());

    app.dispatch(Action::OpenBenchmark);
    app.dispatch(Action::SetQuery {
        query: "rust".into(),
    });
    app.dispatch(Action::SetQuery {
        query: "rust".into(),
    }); // no-op: no update
    app.dispatch(Action::ToggleFavorite { item_id: 1 });

    let ups = rec.0.lock().unwrap();
    assert_eq!(ups.len(), 3);
    // Navigation sends only the stack (+ details), never home/benchmark/info.
    assert!(ups[0].stack.is_some() && ups[0].home.is_none());
    assert!(ups[0].benchmark.is_none() && ups[0].info.is_none());
    assert_eq!(
        ups[0].stack.as_ref().unwrap().last().unwrap().route,
        Route::Benchmark
    );
    // Search sends only home.
    assert!(ups[1].home.is_some() && ups[1].stack.is_none() && ups[1].benchmark.is_none());
    // Favorite changes home and details only.
    assert!(ups[2].home.is_some() && ups[2].details.is_some() && ups[2].benchmark.is_none());
    assert!(ups[0].revision < ups[1].revision && ups[1].revision < ups[2].revision);
}

#[test]
fn perf_metrics_are_recorded() {
    let app = App::new();
    app.dispatch(Action::RecordPerf {
        id: "launch".into(),
        label: "Launch".into(),
        value: 420.0,
        unit: "ms".into(),
        detail: String::new(),
    });
    app.dispatch(Action::OpenInfo);
    let perf = app.state().perf;
    assert!(perf.iter().any(|p| p.id == "launch" && p.value == 420.0));
    assert!(perf.iter().any(|p| p.id == "rust_dispatch"));
}

struct FakeSwift;
impl ExternalBenchRunner for FakeSwift {
    fn language(&self) -> String {
        "Swift".into()
    }
    fn run(&self, bench_id: &str, param: u64, iterations: u32) -> Option<Measurement> {
        // Run the real Rust workload and report it 2× slower: tests the ratio logic.
        let (avg, min, checksum) = app_core::workloads::measure(bench_id, param, 1)?;
        Some(Measurement {
            avg_ms: avg * 2.0 + 0.01,
            min_ms: min * 2.0,
            iterations,
            checksum,
        })
    }
}

#[test]
fn benchmark_runs_in_background_and_compares() {
    let app = App::new();
    app.set_external_runner(Arc::new(FakeSwift));
    app.dispatch(Action::RunBenchmark { id: "fib".into() });
    // dispatch returns immediately; work happens in the background.
    assert!(app.state().benchmark.running);
    // A second request while running is ignored.
    app.dispatch(Action::RunAllBenchmarks);
    wait_idle(&app);

    let s = app.state().benchmark;
    let fib = s.rows.iter().find(|r| r.id == "fib").unwrap();
    assert_eq!(fib.status, BenchStatus::Done);
    assert_eq!(fib.rust.as_ref().unwrap().checksum, 832_040);
    assert_eq!(fib.checksum_match, Some(true));
    assert!(fib.ratio.unwrap() > 1.0);
    assert!(fib.verdict.as_ref().unwrap().starts_with("Rust"));
    assert!(s
        .rows
        .iter()
        .filter(|r| r.id != "fib")
        .all(|r| r.status == BenchStatus::Idle));
    assert!(s.summary.is_some());
    assert_eq!(s.progress, 1.0);
}

#[test]
fn parallel_variant_is_measured_and_matches() {
    let app = App::new();
    app.set_external_runner(Arc::new(FakeSwift));
    app.dispatch(Action::RunBenchmark {
        id: "matmul".into(),
    });
    wait_idle(&app);
    let s = app.state().benchmark;
    let row = s.rows.iter().find(|r| r.id == "matmul").unwrap();
    let par = row.rust_parallel.as_ref().expect("parallel measured");
    assert_eq!(par.checksum, row.rust.as_ref().unwrap().checksum);
    assert!(row.parallel_speedup.is_some());
    assert_eq!(row.checksum_match, Some(true));
}

#[test]
fn info_screen_and_media_featured() {
    let app = App::new();
    app.dispatch(Action::OpenInfo);
    let s = app.state();
    assert_eq!(s.current, Route::Info);
    assert!(s.info.categories.len() >= 10);
    // No numbers until something was measured.
    assert!(s.info.measured.is_empty());
    // Media tests come first.
    assert!(s
        .benchmark
        .rows
        .iter()
        .take(5)
        .all(|r| r.category == "Media"));
    assert_eq!(s.benchmark.rows[0].id, "raytrace");
}
