//! Sayfa akışlarını arayüz olmadan test eder (Discord modelinin büyük avantajı).

use app_core::{
    Action, App, AppObserver, BenchStatus, ExternalBenchRunner, Measurement, Route, ViewState,
};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Default)]
struct Recorder(Mutex<Vec<ViewState>>);
impl AppObserver for Recorder {
    fn on_state_changed(&self, state: ViewState) {
        self.0.lock().unwrap().push(state);
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
    assert_eq!(s.details[0].item.title, "ESP32-C6 Sensör Düğümü");

    // Çift dokunma aynı ekranı iki kez açmaz.
    app.dispatch(Action::OpenDetail { item_id: 2 });
    assert_eq!(app.state().stack.len(), 2);

    app.dispatch(Action::Back);
    assert_eq!(app.state().current, Route::Home);
    // Kökten geri gidilemez.
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
        query: "yok-boyle-bir-sey".into(),
    });
    assert!(app.state().home.rows.is_empty());
    assert!(app.state().home.empty_message.is_some());

    app.dispatch(Action::SetQuery {
        query: String::new(),
    });
    app.dispatch(Action::ToggleFavorite { item_id: 5 });
    let s = app.state();
    assert_eq!(s.home.rows[0].id, 5, "favoriler üstte");
    assert_eq!(s.home.favorite_count, 1);
}

#[test]
fn observer_receives_every_change() {
    let app = App::new();
    let rec = Arc::new(Recorder::default());
    app.set_observer(rec.clone());
    app.dispatch(Action::OpenBenchmark);
    let states = rec.0.lock().unwrap();
    assert!(states.len() >= 2);
    assert_eq!(states.last().unwrap().current, Route::Benchmark);
}

struct FakeSwift;
impl ExternalBenchRunner for FakeSwift {
    fn language(&self) -> String {
        "Swift".into()
    }
    fn run(&self, bench_id: &str, param: u64, iterations: u32) -> Option<Measurement> {
        // Gerçek Rust iş yükünü çağırıp 2× yavaş gibi raporla: oran mantığını test eder.
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
    // dispatch hemen döner; iş arka planda.
    assert!(app.state().benchmark.running);
    // Çalışırken ikinci istek yok sayılır.
    app.dispatch(Action::RunAllBenchmarks);

    let deadline = Instant::now() + Duration::from_secs(60);
    while app.state().benchmark.running {
        assert!(Instant::now() < deadline, "benchmark zaman aşımı");
        std::thread::sleep(Duration::from_millis(5));
    }
    let s = app.state().benchmark;
    let fib = s.rows.iter().find(|r| r.id == "fib").unwrap();
    assert_eq!(fib.status, BenchStatus::Done);
    assert_eq!(fib.rust.as_ref().unwrap().checksum, 832_040);
    assert_eq!(fib.checksum_match, Some(true));
    assert!(fib.ratio.unwrap() > 1.0);
    assert!(fib.verdict.as_ref().unwrap().starts_with("Rust"));
    // Diğerleri çalışmadı.
    assert!(s
        .rows
        .iter()
        .filter(|r| r.id != "fib")
        .all(|r| r.status == BenchStatus::Idle));
    assert!(s.summary.is_some());
    assert_eq!(s.progress, 1.0);
}
