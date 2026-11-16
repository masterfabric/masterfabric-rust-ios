//! Host timing of every benchmark spec (Rust side): `cargo run --release --example calibrate`.
use app_core::screens::benchmark::specs;
use std::time::Instant;

fn main() {
    let filter: Option<String> = std::env::args().nth(1);
    let mut total = 0.0;
    for s in specs() {
        if filter.as_deref().is_some_and(|f| !s.id.contains(f)) {
            continue;
        }
        let t = Instant::now();
        let c = app_core::workloads::run_once(s.id, s.param).unwrap();
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        let par = if s.parallel {
            let t = Instant::now();
            let p = app_core::workloads::run_once_parallel(s.id, s.param).unwrap();
            assert_eq!(p, c, "{} parallel checksum", s.id);
            format!("{:8.2}", t.elapsed().as_secs_f64() * 1000.0)
        } else {
            "       -".into()
        };
        total += ms * (s.iterations as f64 + 1.0);
        println!(
            "{:<12} {:<11} {:>9} {:9.2} ms  par {} ms  chk {:016x}",
            s.id, s.category, s.param, ms, par, c
        );
    }
    println!(
        "est. Rust single-thread total (warm-up + iterations): {:.0} ms",
        total
    );
}
