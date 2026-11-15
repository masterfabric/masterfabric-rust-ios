//! Benchmark ekranının durumu ve iş kuralları.
//! Hangi testlerin olduğu, parametreleri, sonuçların yorumu Rust'ta tanımlıdır.

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
}

/// Tek kaynak: testler ve parametreleri.
pub fn specs() -> Vec<BenchSpec> {
    vec![
        BenchSpec {
            id: workloads::FIB,
            name: "Fibonacci (özyineleme)",
            description: "fib(30) — fonksiyon çağrısı yükü",
            symbol: "function",
            param: 30,
            param_label: "n = 30",
            iterations: 10,
        },
        BenchSpec {
            id: workloads::SORT,
            name: "1M tamsayı sıralama",
            description: "xorshift ile üret + sırala",
            symbol: "arrow.up.arrow.down",
            param: 1_000_000,
            param_label: "1.000.000 × u32",
            iterations: 5,
        },
        BenchSpec {
            id: workloads::SHA256,
            name: "SHA-256",
            description: "1 MiB tampon, elle yazılmış (iki dilde aynı)",
            symbol: "number.square",
            param: 1 << 20,
            param_label: "1 MiB",
            iterations: 10,
        },
        BenchSpec {
            id: workloads::JSON,
            name: "JSON yaz + oku",
            description: "serde_json vs Codable, 5.000 kayıt",
            symbol: "curlybraces",
            param: 5_000,
            param_label: "5.000 kayıt",
            iterations: 5,
        },
        BenchSpec {
            id: workloads::SIEVE,
            name: "Asal elek",
            description: "Eratosthenes, 5.000.000'a kadar",
            symbol: "square.grid.3x3",
            param: 5_000_000,
            param_label: "n = 5.000.000",
            iterations: 10,
        },
        BenchSpec {
            id: workloads::MATMUL,
            name: "Matris çarpımı",
            description: "200×200 f64, i-k-j döngüsü",
            symbol: "tablecells",
            param: 200,
            param_label: "200 × 200",
            iterations: 10,
        },
        BenchSpec {
            id: workloads::HASHMAP,
            name: "HashMap sayımı",
            description: "300.000 ekleme, 65.536 anahtar",
            symbol: "tray.full",
            param: 300_000,
            param_label: "300.000 ekleme",
            iterations: 10,
        },
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
    pub status: BenchStatus,
    pub rust: Option<Measurement>,
    pub swift: Option<Measurement>,
    /// swift_ortalama / rust_ortalama. >1 ise Rust daha hızlı.
    pub ratio: Option<f64>,
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
            status: BenchStatus::Idle,
            rust: None,
            swift: None,
            ratio: None,
            verdict: None,
            checksum_match: None,
        }
    }

    /// Her iki ölçüm de geldiğinde oranı ve yorumu hesapla.
    pub fn finalize(&mut self) {
        match (&self.rust, &self.swift) {
            (Some(r), Some(s)) if r.avg_ms > 0.0 && s.avg_ms > 0.0 => {
                let ratio = s.avg_ms / r.avg_ms;
                self.ratio = Some(ratio);
                self.verdict = Some(verdict(ratio));
                self.checksum_match = Some(r.checksum == s.checksum);
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
        "≈ eşit".to_string()
    } else if ratio > 1.0 {
        format!("Rust {ratio:.2}× hızlı")
    } else {
        format!("Swift {:.2}× hızlı", 1.0 / ratio)
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

    /// Çalıştırma sırasını hazırla. Zaten çalışıyorsa ya da id geçersizse `None`.
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
        for id in &ids {
            if let Some(row) = self.row_mut(id) {
                row.status = BenchStatus::Queued;
                row.rust = None;
                row.swift = None;
                row.finalize();
            }
        }
        self.running = true;
        self.steps_total = ids.len() as u32 * 2;
        self.steps_done = 0;
        self.current_label = None;
        Some(ids)
    }

    pub fn reset(&mut self) {
        if !self.running {
            *self = Self::default();
        }
    }

    /// Tamamlanan testlerin oranlarının geometrik ortalaması.
    pub fn summary(&self) -> Option<String> {
        let ratios: Vec<f64> = self.rows.iter().filter_map(|r| r.ratio).collect();
        if ratios.is_empty() {
            return None;
        }
        let geo = (ratios.iter().map(|r| r.ln()).sum::<f64>() / ratios.len() as f64).exp();
        Some(format!(
            "Geometrik ortalama ({} test): {}",
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
                self.steps_done as f64 / self.steps_total as f64
            },
            current_label: self.current_label.clone(),
            last_total_ms: self.last_total_ms,
            summary: self.summary(),
            rust_profile: crate::rust_profile().to_string(),
            core_info: crate::build_info(),
            swift_runner_attached,
        }
    }
}
