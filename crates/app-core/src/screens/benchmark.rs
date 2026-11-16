//! Benchmark screen state and business rules: which tests exist, their parameters,
//! categories, order, and how results are interpreted all live in Rust.

use crate::app::Measurement;
use crate::ffi_bench::{FfiBenchState, FfiSectionView};
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

pub const IMAGE: &str = "Image";
pub const VIDEO: &str = "Video";
pub const D3: &str = "3D";
pub const MATH: &str = "Math";
pub const DATA: &str = "Data";
pub const TEXT: &str = "Text";
pub const CRYPTO: &str = "Crypto";
pub const ALGO: &str = "Algorithms";
/// Category order on screen.
pub const CATEGORIES: [&str; 8] = [IMAGE, VIDEO, D3, MATH, DATA, TEXT, CRYPTO, ALGO];

/// Single source of truth for tests and parameters, grouped by category.
pub fn specs() -> Vec<BenchSpec> {
    use crate::heavy;
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
            IMAGE,
            workloads::IMAGE,
            "Image pipeline",
            "1024² RGB: gaussian blur → bilinear 512² → color LUT → YUV",
            "photo.on.rectangle.angled",
            1024,
            "1024 × 1024",
            3,
            true,
        ),
        spec(
            IMAGE,
            heavy::MANDELBROT,
            "Mandelbrot",
            "escape-time fractal, 256 max iterations, f64",
            "circle.hexagongrid.fill",
            768,
            "768 × 768",
            3,
            true,
        ),
        spec(
            IMAGE,
            heavy::CONVOLVE,
            "Convolution + edges",
            "5×5 gaussian kernel, then 3×3 Sobel magnitude",
            "square.grid.3x3.fill",
            1536,
            "1536 × 1536",
            3,
            true,
        ),
        spec(
            IMAGE,
            heavy::LANCZOS,
            "Lanczos-3 resize",
            "separable windowed-sinc downscale 1024² → 768²",
            "arrow.down.right.and.arrow.up.left",
            1024,
            "1024² → 768²",
            3,
            true,
        ),
        spec(
            VIDEO,
            workloads::VIDEO,
            "Video processing",
            "30 frames 640×360: YUV + blur + frame diff + motion estimation",
            "film.stack",
            30,
            "30 frames",
            3,
            true,
        ),
        spec(
            VIDEO,
            heavy::DCT,
            "DCT encode",
            "8×8 DCT-II + JPEG quantization, 640×360 frames",
            "square.grid.4x3.fill",
            16,
            "16 frames",
            3,
            true,
        ),
        spec(
            VIDEO,
            heavy::GRADE,
            "Color grade",
            "1280×720 RGB: gamma LUT, saturation, vignette, histogram",
            "camera.filters",
            8,
            "8 frames 720p",
            3,
            true,
        ),
        spec(
            D3,
            workloads::RAYTRACE,
            "Ray tracing",
            "4 spheres + floor, shadows, 2 reflections, 4 samples/pixel",
            "camera.aperture",
            256,
            "256 × 256",
            3,
            true,
        ),
        spec(
            D3,
            heavy::PATHTRACE,
            "Path tracing",
            "diffuse global illumination, 8 spp, 4 bounces",
            "light.max",
            160,
            "160 × 160 · 8 spp",
            3,
            true,
        ),
        spec(
            D3,
            workloads::RASTER,
            "Triangle rasterization",
            "3,000 triangles, z-buffer, 512 × 512",
            "triangle.fill",
            3_000,
            "3,000 triangles",
            5,
            true,
        ),
        spec(
            D3,
            heavy::BVH,
            "BVH build",
            "median split on widest centroid axis",
            "square.stack.3d.down.right.fill",
            50_000,
            "50,000 triangles",
            3,
            true,
        ),
        spec(
            D3,
            heavy::ISOSURFACE,
            "Isosurface extraction",
            "surface nets over 3 metaballs (marching-cubes family)",
            "drop.fill",
            128,
            "128³ cells",
            3,
            true,
        ),
        spec(
            D3,
            workloads::MESH,
            "3D mesh generation",
            "icosphere (5 levels) + displacement + normals + OBJ export",
            "cube.transparent",
            5,
            "20,480 triangles",
            5,
            false,
        ),
        spec(
            MATH,
            workloads::MATMUL,
            "Matrix multiply",
            "f64 dense matrix product (i-k-j order)",
            "square.grid.3x3",
            512,
            "512 × 512",
            2,
            true,
        ),
        spec(
            MATH,
            heavy::NBODY,
            "N-body",
            "5-body solar system, dt = 0.01, energy check",
            "globe.europe.africa.fill",
            300_000,
            "300,000 steps",
            3,
            false,
        ),
        spec(
            MATH,
            heavy::FFT,
            "FFT",
            "iterative radix-2 complex FFT, f64",
            "waveform",
            262_144,
            "2¹⁸ points",
            5,
            false,
        ),
        spec(
            MATH,
            heavy::BIGINT,
            "Big-int factorial",
            "n! with base-10⁹ limbs + digit sum",
            "textformat.123",
            10_000,
            "10,000!",
            3,
            false,
        ),
        spec(
            MATH,
            heavy::PARTICLES,
            "Particle physics",
            "gravity, wall bounces, drag · 50 steps",
            "aqi.medium",
            200_000,
            "200,000 particles",
            3,
            true,
        ),
        spec(
            MATH,
            workloads::FIB,
            "Fibonacci (recursion)",
            "fib(30): function call overhead",
            "function",
            30,
            "n = 30",
            10,
            false,
        ),
        spec(
            MATH,
            workloads::SIEVE,
            "Prime sieve",
            "Sieve of Eratosthenes, count primes",
            "number",
            5_000_000,
            "n = 5,000,000",
            10,
            false,
        ),
        spec(
            DATA,
            workloads::SORT,
            "Sort 1M integers",
            "generate with xorshift + sort",
            "arrow.up.arrow.down",
            1_000_000,
            "1,000,000 × u32",
            5,
            true,
        ),
        spec(
            DATA,
            workloads::JSON,
            "JSON encode + decode",
            "serde_json vs Codable, 5,000 records",
            "curlybraces",
            5_000,
            "5,000 records",
            5,
            false,
        ),
        spec(
            DATA,
            workloads::HASHMAP,
            "HashMap counting",
            "insert / update counters",
            "tablecells",
            300_000,
            "300,000 inserts",
            10,
            false,
        ),
        spec(
            DATA,
            heavy::KMEANS,
            "k-means clustering",
            "k = 16, 8 iterations, 2-D points",
            "circle.grid.cross.fill",
            200_000,
            "200,000 points",
            3,
            true,
        ),
        spec(
            DATA,
            heavy::LZ77,
            "LZ77 compression",
            "hash chains, 32 KiB window, 64 KiB blocks",
            "archivebox.fill",
            1_048_576,
            "1 MiB text",
            3,
            true,
        ),
        spec(
            TEXT,
            heavy::WORDFREQ,
            "Word frequency",
            "case-fold, split, count in a hash map",
            "text.word.spacing",
            2_097_152,
            "2 MiB text",
            3,
            false,
        ),
        spec(
            TEXT,
            heavy::TEXTSEARCH,
            "Regex-like search",
            "5 patterns with . * ^ $ over every line",
            "text.magnifyingglass",
            1_048_576,
            "1 MiB text",
            3,
            true,
        ),
        spec(
            TEXT,
            heavy::STRSORT,
            "String sort",
            "generate + sort lowercase strings",
            "textformat.abc",
            200_000,
            "200,000 strings",
            3,
            true,
        ),
        spec(
            TEXT,
            heavy::LEVENSHTEIN,
            "Edit distance",
            "Levenshtein DP on 8 DNA-like string pairs",
            "character.cursor.ibeam",
            1_600,
            "8 × 1,600 chars",
            3,
            true,
        ),
        spec(
            CRYPTO,
            workloads::SHA256,
            "SHA-256",
            "hand-written in both languages (no CryptoKit)",
            "number.square",
            4_194_304,
            "4 MiB",
            5,
            false,
        ),
        spec(
            CRYPTO,
            heavy::CHACHA20,
            "ChaCha20",
            "RFC 8439 stream cipher encryption",
            "lock.fill",
            4_194_304,
            "4 MiB",
            3,
            true,
        ),
        spec(
            CRYPTO,
            heavy::CRC32,
            "CRC-32",
            "table-driven IEEE checksum",
            "checkmark.shield.fill",
            8_388_608,
            "8 MiB",
            5,
            false,
        ),
        spec(
            ALGO,
            heavy::DIJKSTRA,
            "Dijkstra",
            "binary-heap shortest paths on a weighted grid",
            "point.topleft.down.to.point.bottomright.curvepath",
            512,
            "512 × 512 grid",
            3,
            false,
        ),
        spec(
            ALGO,
            heavy::BFS,
            "Graph BFS",
            "random graph, CSR adjacency",
            "point.3.connected.trianglepath.dotted",
            300_000,
            "300k nodes · 1.2M edges",
            3,
            false,
        ),
        spec(
            ALGO,
            heavy::NQUEENS,
            "N-queens",
            "bitmask backtracking, count all solutions",
            "crown.fill",
            13,
            "n = 13",
            3,
            true,
        ),
        spec(
            ALGO,
            heavy::SUDOKU,
            "Sudoku solver",
            "backtracking + fewest-candidates, 4 hard puzzles",
            "square.grid.3x3.square",
            40,
            "160 puzzles",
            3,
            true,
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
    /// Test #37, its own section (not one of the 8 compute categories).
    pub ffi: FfiSectionView,
}

#[derive(Clone, Debug)]
pub struct BenchmarkState {
    pub rows: Vec<BenchRow>,
    pub running: bool,
    pub steps_total: u32,
    pub steps_done: u32,
    pub current_label: Option<String>,
    pub last_total_ms: Option<f64>,
    pub ffi: FfiBenchState,
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
            ffi: FfiBenchState::default(),
        }
    }
}

impl BenchmarkState {
    /// True while either the compute tests or the FFI section are running.
    pub fn busy(&self) -> bool {
        self.running || self.ffi.running
    }

    pub fn row_mut(&mut self, id: &str) -> Option<&mut BenchRow> {
        self.rows.iter_mut().find(|r| r.id == id)
    }

    /// Prepare a run. `None` if already running or no valid ids.
    pub fn begin(&mut self, ids: Vec<String>) -> Option<Vec<String>> {
        if self.busy() {
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
        if !self.busy() {
            *self = Self::default();
        }
    }

    /// Geometric mean of the completed single-thread ratios.
    pub fn summary(&self) -> Option<String> {
        let (geo, n) = self.geomean()?;
        Some(format!("Geometric mean ({n} tests): {}", verdict(geo)))
    }

    /// Geometric mean of the completed single-thread Swift/Rust ratios, and their count.
    pub fn geomean(&self) -> Option<(f64, usize)> {
        let ratios: Vec<f64> = self.rows.iter().filter_map(|r| r.ratio).collect();
        if ratios.is_empty() {
            return None;
        }
        let geo = (ratios.iter().map(|r| r.ln()).sum::<f64>() / ratios.len() as f64).exp();
        Some((geo, ratios.len()))
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
            ffi: self.ffi.view(self.geomean().map(|g| g.0)),
        }
    }
}
