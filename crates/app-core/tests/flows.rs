//! Tests screen flows without any UI (a big win of the shared-core architecture).

use app_core::{
    Action, App, AppObserver, BenchStatus, ExternalBenchRunner, FfiSample, Measurement, Route,
    ViewUpdate,
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
    fn run_ffi(
        &self,
        case_id: &str,
        variant: &str,
        calls: u64,
        _reps: u64,
        _samples: u32,
    ) -> Option<FfiSample> {
        // Real reference checksum; per-call pretends to pay 20 ns per crossing.
        let checksum = app_core::ffi_bench::reference_checksum(case_id, calls)?;
        let ns = match variant {
            "swift_inline" => 1.0,
            "rust_per_call" => 21.0,
            _ => 1.0 + 20.0 / calls as f64,
        };
        Some(FfiSample {
            ns_per_call: ns,
            checksum,
        })
    }
}

#[test]
fn ffi_section_runs_separately_and_reports_crossing_cost() {
    let app = App::new();
    app.set_external_runner(Arc::new(FakeSwift));
    app.dispatch(Action::RunFfiBench);
    assert!(app.state().benchmark.ffi.running);
    // Compute tests cannot start while the FFI section runs.
    app.dispatch(Action::RunBenchmark { id: "fib".into() });
    let deadline = Instant::now() + Duration::from_secs(60);
    while app.state().benchmark.ffi.running {
        assert!(Instant::now() < deadline, "ffi bench timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
    let b = app.state().benchmark;
    assert!(b.rows.iter().all(|r| r.status == BenchStatus::Idle));
    let ffi = b.ffi;
    assert_eq!(ffi.rows.len(), 9);
    assert!(ffi.rows.iter().all(|r| r.checksum_match == Some(true)));
    assert_eq!(ffi.crossing_ns, Some(20.0));
    assert!(ffi.summary.unwrap().contains("Break-even"));
    assert_eq!(ffi.progress, 1.0);
    let one = &ffi.rows[0];
    assert_eq!(one.calls, 1);
    assert!(one.rust_batched.is_some());
    assert!(ffi
        .rows
        .iter()
        .filter(|r| r.group == "marshal")
        .all(|r| r.rust_batched.is_none()));
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
fn info_screen_and_categories() {
    let app = App::new();
    app.dispatch(Action::OpenInfo);
    let s = app.state();
    assert_eq!(s.current, Route::Info);
    assert!(s.info.categories.len() >= 10);
    // No numbers until something was measured.
    assert!(s.info.measured.is_empty());
    // 30+ tests, grouped by category in the fixed order.
    let rows = &s.benchmark.rows;
    assert!(rows.len() >= 30);
    let mut seen: Vec<&str> = Vec::new();
    for r in rows {
        if seen.last() != Some(&r.category.as_str()) {
            assert!(
                !seen.contains(&r.category.as_str()),
                "category {} is split",
                r.category
            );
            seen.push(&r.category);
        }
    }
    assert_eq!(
        seen,
        [
            "Image",
            "Video",
            "3D",
            "Math",
            "Data",
            "Text",
            "Crypto",
            "Algorithms"
        ]
    );
    let mut ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), rows.len(), "duplicate benchmark id");
}

#[test]
fn run_category_only_runs_that_category() {
    let app = App::new();
    app.dispatch(Action::RunCategory {
        category: "Crypto".into(),
    });
    wait_idle(&app);
    let s = app.state().benchmark;
    for r in &s.rows {
        assert_eq!(r.rust.is_some(), r.category == "Crypto", "{}", r.id);
    }
}

#[test]
fn every_benchmark_runs_and_parallel_matches() {
    for spec in app_core::screens::benchmark::specs() {
        let c = app_core::workloads::run_once(spec.id, spec.param).expect(spec.id);
        if spec.parallel {
            let p = app_core::workloads::run_once_parallel(spec.id, spec.param).expect(spec.id);
            assert_eq!(c, p, "{} parallel checksum", spec.id);
        }
    }
}
