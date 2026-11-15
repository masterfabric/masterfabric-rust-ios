//! "What can you build with Rust?" info screen. Content lives in the Rust core;
//! SwiftUI only renders it as cards, a diagram and charts.
//! No made-up statistics: the only numbers shown are this app's own measurements.

use crate::screens::benchmark::BenchmarkState;

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectCategory {
    pub id: String,
    pub title: String,
    pub symbol: String,
    /// Color key mapped to the platform palette: orange, blue, green, purple, teal, pink, indigo, red, mint, brown, cyan
    pub tint: String,
    pub summary: String,
    pub examples: Vec<String>,
    pub crates: Vec<String>,
    /// Benchmark ids in this app that represent this area.
    pub related_bench_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArchStep {
    pub title: String,
    pub detail: String,
    pub symbol: String,
    /// "swift" or "rust": which diagram layer it belongs to.
    pub layer: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeasuredRatio {
    pub bench_id: String,
    pub name: String,
    pub category: String,
    pub rust_ms: f64,
    pub swift_ms: f64,
    pub ratio: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InfoViewModel {
    pub title: String,
    pub intro: String,
    pub architecture: Vec<ArchStep>,
    pub categories: Vec<ProjectCategory>,
    pub measured: Vec<MeasuredRatio>,
    pub measured_note: String,
    pub strengths: Vec<String>,
    pub tradeoffs: Vec<String>,
}

fn s(v: &str) -> String {
    v.to_string()
}
fn v(items: &[&str]) -> Vec<String> {
    items.iter().map(|x| x.to_string()).collect()
}

#[allow(clippy::too_many_arguments)]
fn cat(
    id: &str,
    title: &str,
    symbol: &str,
    tint: &str,
    summary: &str,
    examples: &[&str],
    crates: &[&str],
    bench: &[&str],
) -> ProjectCategory {
    ProjectCategory {
        id: s(id),
        title: s(title),
        symbol: s(symbol),
        tint: s(tint),
        summary: s(summary),
        examples: v(examples),
        crates: v(crates),
        related_bench_ids: v(bench),
    }
}

impl InfoViewModel {
    pub fn categories() -> Vec<ProjectCategory> {
        vec![
            cat("mobile", "Shared mobile core", "iphone.gen3", "orange",
                "Write business logic once in Rust; iOS, Android and Flutter use it behind native UI. This app is built this way.",
                &["State + navigation core", "Encryption / sync", "Offline data layer"],
                &["uniffi", "flutter_rust_bridge", "tokio"], &["json", "hashmap"]),
            cat("media", "Media processing", "photo.stack", "pink",
                "Image filters, video frame analysis, audio processing and encoders; C speed with memory safety.",
                &["Photo filter engine", "Video thumbnails / motion detection", "Camera stream processing"],
                &["image", "rav1e", "symphonia"], &["image", "video", "raytrace"]),
            cat("graphics", "3D & games (Bevy)", "cube.transparent", "purple",
                "ECS game engine, procedural model generation, GPU programming.",
                &["2D/3D games", "Procedural mesh generation", "Physics simulation"],
                &["bevy", "wgpu", "glam"], &["mesh", "raster", "raytrace"]),
            cat("cli", "CLI tools", "terminal.fill", "teal",
                "Single-binary, dependency-free command-line tools that start instantly.",
                &["Log / CSV scanner", "Code generator", "Developer tooling"],
                &["clap", "rayon", "indicatif"], &["sort", "sieve"]),
            cat("desktop", "Desktop (Tauri)", "macwindow", "indigo",
                "Web UI + Rust backend; small bundles for macOS, Windows and Linux.",
                &["Local AI dashboard", "File manager", "Internal tools"],
                &["tauri", "serde"], &[]),
            cat("backend", "Backend (Axum)", "server.rack", "blue",
                "Async web services and APIs with no garbage-collector pauses.",
                &["REST / gRPC API", "WebSocket server", "Queue workers"],
                &["axum", "tokio", "sqlx"], &["json"]),
            cat("wasm", "WebAssembly", "globe", "cyan",
                "Run Rust code in the browser or at the edge.",
                &["In-browser image processing", "Edge functions", "Web games"],
                &["wasm-bindgen", "wasm-pack", "leptos"], &["sha256"]),
            cat("embedded", "Embedded systems", "cpu", "green",
                "Bare-metal (no_std) on microcontrollers: ESP32, RP2040/Pico, STM32; on Linux on a Raspberry Pi.",
                &["ESP32 sensor node (BLE)", "Pico motor controller", "Raspberry Pi home automation"],
                &["embassy", "esp-hal", "embassy-rp", "rppal"], &[]),
            cat("robotics", "Robotics", "figure.walk.motion", "red",
                "Pi 5 as the brain (camera, path finding) + Pico/ESP32 as the muscle (motors, sensors).",
                &["Obstacle-avoiding robot", "Line / object tracking", "Phone control"],
                &["ros2-rust", "nalgebra", "embassy"], &["matmul", "video"]),
            cat("network", "Networking", "network", "mint",
                "Proxies, VPN components, QUIC, port scanners and connection monitoring.",
                &["Local network monitor", "Fast file transfer", "Proxy / tunnel"],
                &["tokio", "quinn", "hyper"], &["sha256"]),
            cat("data", "Data processing", "chart.bar.doc.horizontal", "brown",
                "Parallel processing of large CSV/JSON/log files, analytics and search.",
                &["Log analysis", "Local search engine", "ETL pipeline"],
                &["polars", "arrow", "serde_json"], &["json", "sort", "hashmap"]),
        ]
    }

    pub fn architecture() -> Vec<ArchStep> {
        vec![
            ArchStep {
                title: s("SwiftUI screen"),
                detail: s("User taps; the screen only renders"),
                symbol: s("hand.tap.fill"),
                layer: s("swift"),
            },
            ArchStep {
                title: s("dispatch(Action)"),
                detail: s("The event crosses the UniFFI bridge into Rust"),
                symbol: s("arrow.down.circle.fill"),
                layer: s("swift"),
            },
            ArchStep {
                title: s("Rust core"),
                detail: s("AppState updates: navigation, data, business rules, benchmarks"),
                symbol: s("gearshape.2.fill"),
                layer: s("rust"),
            },
            ArchStep {
                title: s("ViewUpdate (partial)"),
                detail: s("Only the changed screen state returns to Swift via StateListener"),
                symbol: s("arrow.up.circle.fill"),
                layer: s("rust"),
            },
            ArchStep {
                title: s("CoreBridge (@Observable)"),
                detail: s("SwiftUI re-renders only views that read the changed part"),
                symbol: s("rectangle.on.rectangle"),
                layer: s("swift"),
            },
        ]
    }

    pub fn build(bench: &BenchmarkState) -> Self {
        let measured: Vec<MeasuredRatio> = bench
            .rows
            .iter()
            .filter_map(|r| {
                let (rs, sw, ratio) = (r.rust.as_ref()?, r.swift.as_ref()?, r.ratio?);
                Some(MeasuredRatio {
                    bench_id: r.id.clone(),
                    name: r.name.clone(),
                    category: r.category.clone(),
                    rust_ms: rs.avg_ms,
                    swift_ms: sw.avg_ms,
                    ratio,
                })
            })
            .collect();
        let measured_note = if measured.is_empty() {
            s("Run the benchmarks first. Only results measured on this device are shown here.")
        } else {
            s("Measured on this device (Swift avg / Rust avg, single-thread). > 1 means Rust is faster. In the Simulator the CPU measured is the Mac's.")
        };
        Self {
            title: s("What can you build with Rust?"),
            intro: s("Rust runs at C/C++ speed, without a garbage collector, and enforces memory safety at compile time. It fits everything from microcontrollers to servers, from shared mobile cores to the browser."),
            architecture: Self::architecture(),
            categories: Self::categories(),
            measured,
            measured_note,
            strengths: v(&["No GC → predictable latency", "Borrow checker → memory bugs caught at compile time", "cargo → build, test, packages in one tool", "Same code: iOS, Android, desktop, server, embedded"]),
            tradeoffs: v(&["Steep learning curve (ownership rules)", "Compile times can be long", "Some hardware SDKs are officially C/C++ only"]),
        }
    }
}
