//! Benchmark screen state and business rules: which tests exist, their parameters,
//! categories, order, and how results are interpreted all live in Rust.

use crate::app::Measurement;
use crate::workloads;

#[derive(Clone, Debug, PartialEq)]
pub enum BenchStatus {
    Idle,
    Queued,
    RunningRust,
    RunningSwift,
    Done,
    Failed { message: String },
}

#[derive(Clone, Debug)]
pub struct BenchSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub symbol: &'static str,
    pub param: u64,
    pub param_label: &'static str,
    pub iterations: u32,
    pub category: &'static str,
    /// Also measured with a multi-threaded (rayon) Rust variant.
    pub parallel: bool,
}

pub const MEDIA: &str = "Media";
pub const GENERAL: &str = "General";

/// Single source of truth for tests and parameters. Media is the featured category (first).
pub fn specs() -> Vec<BenchSpec> {
    use workloads::*;
    #[allow(clippy::too_many_arguments)]
    fn spec(
        category: &'static str,
        id: &'static str,
        name: &'static str,
        description: &'static str,
        symbol: &'static str,
        param: u64,
        param_label: &'static str,
        iterations: u32,
        parallel: bool,
    ) -> BenchSpec {
        BenchSpec {
            id,
            name,
            description,
            symbol,
            param,
            param_label,
            iterations,
            category,
            parallel,
        }
    }
    vec![
        spec(
            MEDIA,
            RAYTRACE,
            "Ray tracing",
            "4 spheres + floor, shadows, 2 reflections, 4 samples/pixel",
            "camera.aperture",
            256,
            "256 × 256",
            3,
            true,
        ),
        spec(
            MEDIA,
            RASTER,
            "Triangle rasterization",
            "3,000 triangles, z-buffer, 512 × 512",
            "triangle.fill",
            3_000,
            "3,000 triangles",
            5,
            true,
        ),
        spec(
            MEDIA,
            IMAGE,
            "Image processing",
            "1024² RGB: gaussian blur → bilinear 512² → color LUT → YUV",
            "photo.on.rectangle.angled",
            1024,
            "1024 × 1024",
            3,
            true,
        ),
        spec(
            MEDIA,
            VIDEO,
            "Video processing",
            "30 frames 640×360: YUV + blur + frame diff + motion estimation",
            "film.stack",
            30,
            "30 frames",
            3,
            true,
        ),
        spec(
            MEDIA,
            MESH,
            "3D model generation",
            "icosphere (5 levels) + displacement + normals + OBJ export",
            "cube.transparent",
            5,
            "20,480 triangles",
            5,
            false,
        ),
        spec(
            GENERAL,
            FIB,
            "Fibonacci (recursion)",
            "fib(30) — function call overhead",
            "function",
            30,
            "n = 30",
            10,
            false,
        ),
        spec(
            GENERAL,
            SORT,
            "Sort 1M integers",
            "generate with xorshift + sort",
            "arrow.up.arrow.down",
            1_000_000,
            "1,000,000 × u32",
            5,
            true,
        ),
        spec(
            GENERAL,
            SHA256,
            "SHA-256",
            "1 MiB buffer, hand-written (same in both languages)",
            "number.square",
            1 << 20,
            "1 MiB",
            10,
            false,
        ),
        spec(
            GENERAL,
            JSON,
            "JSON encode + decode",
            "serde_json vs Codable, 5,000 records",
            "curlybraces",
            5_000,
            "5,000 records",
            5,
            false,
        ),
        spec(
            GENERAL,
            SIEVE,
            "Prime sieve",
            "Eratosthenes up to 5,000,000",
            "square.grid.3x3",
            5_000_000,
            "n = 5,000,000",
            10,
            false,
        ),
        spec(
            GENERAL,
            MATMUL,
            "Matrix multiply",
            "200×200 f64, i-k-j loop order",
            "tablecells",
            200,
            "200 × 200",
            10,
            true,
        ),
        spec(
            GENERAL,
            HASHMAP,
            "HashMap counting",
            "300,000 inserts, 65,536 keys",
            "tray.full",
            300_000,
            "300,000 inserts",
            10,
            false,
        ),
    ]
}

#[derive(Clone, Debug, PartialEq)]
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
    /// Single-thread Rust (apples-to-apples with Swift).
    pub rust: Option<Measurement>,
    /// Multi-threaded Rust (rayon). Not used in the Rust-vs-Swift ratio.
    pub rust_parallel: Option<Measurement>,
    pub swift: Option<Measurement>,
    /// swift_avg / rust_avg (single-thread). > 1 means Rust is faster.
    pub ratio: Option<f64>,
    /// rust_avg / rust_parallel_avg.
    pub parallel_speedup: Option<f64>,
    pub verdict: Option<String>,
    pub checksum_match: Option<bool>,
}

impl BenchRow {
    fn from_spec(s: &BenchSpec) -> Self {
        Self {
            id: s.id.to_string(),
            name: s.name.to_string(),
            description: s.description.to_string(),
            symbol: s.symbol.to_string(),
            param: s.param,
            param_label: s.param_label.to_string(),
            iterations: s.iterations,
            category: s.category.to_string(),
            has_parallel: s.parallel,
            status: BenchStatus::Idle,
            rust: None,
            rust_parallel: None,
            swift: None,
            ratio: None,
            parallel_speedup: None,
            verdict: None,
            checksum_match: None,
        }
    }

    /// Compute ratio, speedup and verdict once measurements are in.
    pub fn finalize(&mut self) {
        self.parallel_speedup = match (&self.rust, &self.rust_parallel) {
            (Some(r), Some(p)) if p.avg_ms > 0.0 => Some(r.avg_ms / p.avg_ms),
            _ => None,
        };
        match (&self.rust, &self.swift) {
            (Some(r), Some(s)) if r.avg_ms > 0.0 && s.avg_ms > 0.0 => {
                let ratio = s.avg_ms / r.avg_ms;
                self.ratio = Some(ratio);
                self.verdict = Some(verdict(ratio));
                let par_ok = self
                    .rust_parallel
                    .as_ref()
                    .is_none_or(|p| p.checksum == r.checksum);
                self.checksum_match = Some(r.checksum == s.checksum && par_ok);
            }
            _ => {
                self.ratio = None;
                self.verdict = None;
                self.checksum_match = None;
            }
        }
    }
}

pub fn verdict(ratio: f64) -> String {
    if (0.95..=1.05).contains(&ratio) {
        "≈ equal".to_string()
    } else if ratio > 1.0 {
        format!("Rust {ratio:.2}× faster")
    } else {
        format!("Swift {:.2}× faster", 1.0 / ratio)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkViewState {
    pub rows: Vec<BenchRow>,
    pub running: bool,
    /// 0.0 ... 1.0
    pub progress: f64,
    pub current_label: Option<String>,
    pub last_total_ms: Option<f64>,
    pub summary: Option<String>,
    pub rust_profile: String,
    pub core_info: String,
    pub rust_threads: u32,
    pub swift_runner_attached: bool,
}

#[derive(Clone, Debug)]
pub struct BenchmarkState {
    pub rows: Vec<BenchRow>,
    pub running: bool,
    pub steps_total: u32,
    pub steps_done: u32,
    pub current_label: Option<String>,
    pub last_total_ms: Option<f64>,
}

impl Default for BenchmarkState {
    fn default() -> Self {
        Self {
            rows: specs().iter().map(BenchRow::from_spec).collect(),
            running: false,
            steps_total: 0,
            steps_done: 0,
            current_label: None,
            last_total_ms: None,
        }
    }
}

impl BenchmarkState {
    pub fn row_mut(&mut self, id: &str) -> Option<&mut BenchRow> {
        self.rows.iter_mut().find(|r| r.id == id)
    }

    /// Prepare a run. `None` if already running or no valid ids.
    pub fn begin(&mut self, ids: Vec<String>) -> Option<Vec<String>> {
        if self.running {
            return None;
        }
        let ids: Vec<String> = ids
            .into_iter()
            .filter(|id| self.rows.iter().any(|r| &r.id == id))
            .collect();
        if ids.is_empty() {
            return None;
        }
        let mut steps = 0;
        for id in &ids {
            if let Some(row) = self.row_mut(id) {
                row.status = BenchStatus::Queued;
                row.rust = None;
                row.rust_parallel = None;
                row.swift = None;
                row.finalize();
                steps += if row.has_parallel { 3 } else { 2 };
            }
        }
        self.running = true;
        self.steps_total = steps;
        self.steps_done = 0;
        self.current_label = None;
        Some(ids)
    }

    pub fn reset(&mut self) {
        if !self.running {
            *self = Self::default();
        }
    }

    /// Geometric mean of the completed single-thread ratios.
    pub fn summary(&self) -> Option<String> {
        let ratios: Vec<f64> = self.rows.iter().filter_map(|r| r.ratio).collect();
        if ratios.is_empty() {
            return None;
        }
        let geo = (ratios.iter().map(|r| r.ln()).sum::<f64>() / ratios.len() as f64).exp();
        Some(format!(
            "Geometric mean ({} tests): {}",
            ratios.len(),
            verdict(geo)
        ))
    }

    pub fn view(&self, swift_runner_attached: bool) -> BenchmarkViewState {
        BenchmarkViewState {
            rows: self.rows.clone(),
            running: self.running,
            progress: if self.steps_total == 0 {
                if self.rows.iter().any(|r| r.status == BenchStatus::Done) {
                    1.0
                } else {
                    0.0
                }
            } else {
                (self.steps_done as f64 / self.steps_total as f64).min(1.0)
            },
            current_label: self.current_label.clone(),
            last_total_ms: self.last_total_ms,
            summary: self.summary(),
            rust_profile: crate::rust_profile().to_string(),
            core_info: crate::build_info(),
            rust_threads: rayon::current_num_threads() as u32,
            swift_runner_attached,
        }
    }
}
