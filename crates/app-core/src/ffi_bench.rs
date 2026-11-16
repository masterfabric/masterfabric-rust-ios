//! # Test #37: FFI boundary ("chatty calls")
//!
//! The 36 compute benchmarks time each workload *inside* its own language: Rust times
//! `run_once` with `Instant` in Rust, Swift times its twin with `DispatchTime` in Swift.
//! The FFI crossing happens once per benchmark, outside the timed region. That answers
//! "which language computes faster", but not "what does it cost to *call* Rust a lot".
//!
//! This section measures exactly that. The same tiny operation (`mix`, ~1 ns of work) is
//! executed `calls` times in three ways:
//!
//! * **Swift inline**: a plain Swift loop, no FFI at all (the baseline).
//! * **Rust per call**: Swift calls the UniFFI function `ffi_mix` once per operation.
//! * **Rust batched**: Swift makes ONE UniFFI call (`ffi_mix_batch`) that runs the loop in Rust.
//!
//! Plus marshaling cases at a fixed call count that pass a `String`, a small record and a
//! `Vec<u32>` across the boundary on every call.
//!
//! Timing runs on the Swift side (only Swift can observe the Swift→Rust crossing), but the
//! test list, parameters, ordering, interpretation and verdicts live here in Rust.
//! All variants produce the same checksum; mismatches are reported.

/// Section title on the Benchmark screen (not one of the 8 compute categories).
pub const SECTION: &str = "FFI boundary";

/// Call counts of the sweep.
pub const SWEEP: [u64; 6] = [1, 10, 100, 1_000, 10_000, 100_000];
/// Call count for the marshaling cases.
pub const MARSHAL_CALLS: u64 = 10_000;
/// Every timed sample performs ~this many operations (`reps = TARGET_OPS / calls`), so even
/// `calls = 1` is long enough to be well above timer resolution.
pub const TARGET_OPS: u64 = 1_000_000;
/// Timed samples per variant; the median is reported. One extra untimed warm-up sample.
pub const SAMPLES: u32 = 5;

/// Case ids (also the UniFFI function the per-call variant uses).
pub const CASE_U64: &str = "u64";
pub const CASE_STRING: &str = "string";
pub const CASE_RECORD: &str = "record";
pub const CASE_VEC: &str = "vec16";

/// Variant ids.
pub const SWIFT_INLINE: &str = "swift_inline";
pub const RUST_PER_CALL: &str = "rust_per_call";
pub const RUST_BATCHED: &str = "rust_batched";

/// Length of the `Vec<u32>` in the `vec16` case.
pub const VEC_LEN: usize = 16;

// ---------------------------------------------------------------- tiny ops (Swift twins in FfiBench.swift)

/// The "tiny op": one multiply, xor, rotate. Swift twin: `FfiBench.mix`.
#[inline]
pub fn mix(acc: u64, i: u64) -> u64 {
    (acc ^ i).wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(5)
}

/// `n` chained `mix` steps starting at `seed` (the batched variant: one FFI call, n ops).
pub fn mix_batch(seed: u64, n: u64) -> u64 {
    let mut a = seed;
    for i in 0..n {
        a = mix(a, i);
    }
    a
}

/// Folds the UTF-8 bytes of `s` into `seed`.
pub fn str_mix(s: &str, seed: u64) -> u64 {
    s.bytes().fold(seed, |a, b| mix(a, b as u64))
}

/// Mixes a small 3×i32 record.
pub fn point_mix(x: i32, y: i32, z: i32) -> u64 {
    mix(
        mix(mix(0, x as u32 as u64), y as u32 as u64),
        z as u32 as u64,
    )
}

/// Folds a `u32` slice into `seed`.
pub fn vec_mix(v: &[u32], seed: u64) -> u64 {
    v.iter().fold(seed, |a, &x| mix(a, x as u64))
}

/// The 16 strings the `string` case cycles through (Swift uses the same list).
pub fn sample_strings() -> Vec<String> {
    (0..16).map(|i| format!("item-{i}:masterfabric")).collect()
}

/// The `Vec<u32>` the `vec16` case passes on every call.
pub fn sample_vec() -> Vec<u32> {
    (0..VEC_LEN as u32)
        .map(|i| i.wrapping_mul(2_654_435_761))
        .collect()
}

/// Reference checksum of one sample, computed in Rust without any FFI.
/// Every Swift-side variant must return exactly this value.
pub fn reference_checksum(case: &str, calls: u64) -> Option<u64> {
    let reps = reps_for(calls);
    let strings = sample_strings();
    let v = sample_vec();
    let mut total = 0u64;
    for r in 0..reps {
        let seed = 0x5EED ^ r;
        let acc = match case {
            CASE_U64 => mix_batch(seed, calls),
            CASE_STRING => {
                (0..calls).fold(seed, |a, i| mix(a, str_mix(&strings[(i & 15) as usize], i)))
            }
            CASE_RECORD => (0..calls).fold(seed, |a, i| {
                let x = i as i32;
                mix(a, point_mix(x, x.wrapping_mul(3), x.wrapping_neg()))
            }),
            CASE_VEC => (0..calls).fold(seed, |a, i| mix(a, vec_mix(&v, i))),
            _ => return None,
        };
        total ^= acc;
    }
    Some(total)
}

/// Repetitions per timed sample.
pub fn reps_for(calls: u64) -> u64 {
    (TARGET_OPS / calls.max(1)).max(1)
}

// ---------------------------------------------------------------- state

/// One measured variant: median ns per operation over `SAMPLES` samples.
#[derive(Clone, Debug, PartialEq)]
pub struct FfiSample {
    pub ns_per_call: f64,
    pub checksum: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FfiRow {
    pub id: String,
    /// `"sweep"` or `"marshal"`.
    pub group: String,
    pub case_id: String,
    pub label: String,
    pub detail: String,
    pub calls: u64,
    pub reps: u64,
    pub swift_inline: Option<FfiSample>,
    pub rust_per_call: Option<FfiSample>,
    /// Only for the u64 sweep.
    pub rust_batched: Option<FfiSample>,
    pub has_batched: bool,
    pub expected_checksum: u64,
    /// per-call ns − inline ns: what one crossing (incl. marshaling) adds.
    pub overhead_ns: Option<f64>,
    /// per-call ns / inline ns (> 1: Swift inline is faster).
    pub per_call_slowdown: Option<f64>,
    /// batched ns / inline ns.
    pub batched_ratio: Option<f64>,
    pub verdict: Option<String>,
    pub checksum_match: Option<bool>,
    pub running: bool,
}

impl FfiRow {
    fn new(
        group: &str,
        case_id: &str,
        label: String,
        detail: &str,
        calls: u64,
        batched: bool,
    ) -> Self {
        Self {
            id: format!("ffi-{case_id}-{calls}"),
            group: group.into(),
            case_id: case_id.into(),
            label,
            detail: detail.into(),
            calls,
            reps: reps_for(calls),
            swift_inline: None,
            rust_per_call: None,
            rust_batched: None,
            has_batched: batched,
            expected_checksum: reference_checksum(case_id, calls).unwrap_or(0),
            overhead_ns: None,
            per_call_slowdown: None,
            batched_ratio: None,
            verdict: None,
            checksum_match: None,
            running: false,
        }
    }

    pub fn variants(&self) -> Vec<&'static str> {
        let mut v = vec![SWIFT_INLINE, RUST_PER_CALL];
        if self.has_batched {
            v.push(RUST_BATCHED);
        }
        v
    }

    pub fn set(&mut self, variant: &str, s: Option<FfiSample>) {
        match variant {
            SWIFT_INLINE => self.swift_inline = s,
            RUST_PER_CALL => self.rust_per_call = s,
            RUST_BATCHED => self.rust_batched = s,
            _ => {}
        }
    }

    pub fn finalize(&mut self) {
        let inline = self.swift_inline.as_ref().map(|s| s.ns_per_call);
        let per = self.rust_per_call.as_ref().map(|s| s.ns_per_call);
        let bat = self.rust_batched.as_ref().map(|s| s.ns_per_call);
        self.overhead_ns = match (inline, per) {
            (Some(i), Some(p)) => Some((p - i).max(0.0)),
            _ => None,
        };
        self.per_call_slowdown = match (inline, per) {
            (Some(i), Some(p)) if i > 0.0 => Some(p / i),
            _ => None,
        };
        self.batched_ratio = match (inline, bat) {
            (Some(i), Some(b)) if i > 0.0 => Some(b / i),
            _ => None,
        };
        let samples: Vec<&FfiSample> =
            [&self.swift_inline, &self.rust_per_call, &self.rust_batched]
                .into_iter()
                .flatten()
                .collect();
        let complete = self.swift_inline.is_some()
            && self.rust_per_call.is_some()
            && (!self.has_batched || self.rust_batched.is_some());
        self.checksum_match =
            complete.then(|| samples.iter().all(|s| s.checksum == self.expected_checksum));
        self.verdict = match (self.per_call_slowdown, self.batched_ratio) {
            (Some(p), Some(b)) => Some(format!(
                "per call {} · batched {}",
                slowdown_text(p),
                slowdown_text(b)
            )),
            (Some(p), None) => Some(format!("per call {}", slowdown_text(p))),
            _ => None,
        };
    }
}

/// `x` = rust variant time / swift inline time.
pub fn slowdown_text(x: f64) -> String {
    if (0.9..=1.1).contains(&x) {
        "≈ inline Swift".into()
    } else if x > 1.0 {
        format!("{x:.1}× slower than inline Swift")
    } else {
        format!("{:.1}× faster than inline Swift", 1.0 / x)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FfiSectionView {
    pub title: String,
    pub intro: String,
    pub rows: Vec<FfiRow>,
    pub running: bool,
    pub progress: f64,
    pub current_label: Option<String>,
    /// Estimated cost of one Swift→Rust UniFFI call with a u64 argument (ns).
    pub crossing_ns: Option<f64>,
    /// Human-readable crossover analysis.
    pub summary: Option<String>,
    pub last_total_ms: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct FfiBenchState {
    pub rows: Vec<FfiRow>,
    pub running: bool,
    pub steps_total: u32,
    pub steps_done: u32,
    pub current_label: Option<String>,
    pub last_total_ms: Option<f64>,
}

pub fn default_rows() -> Vec<FfiRow> {
    let mut rows: Vec<FfiRow> = SWEEP
        .iter()
        .map(|&n| {
            FfiRow::new(
                "sweep",
                CASE_U64,
                format!("{} call{}", group_digits(n), if n == 1 { "" } else { "s" }),
                "mix(u64, u64) → u64",
                n,
                true,
            )
        })
        .collect();
    rows.push(FfiRow::new(
        "marshal",
        CASE_STRING,
        "String argument".into(),
        "~20-byte String → RustBuffer copy + UTF-8 validation per call",
        MARSHAL_CALLS,
        false,
    ));
    rows.push(FfiRow::new(
        "marshal",
        CASE_RECORD,
        "Record argument".into(),
        "struct { x, y, z: i32 } serialized into a RustBuffer per call",
        MARSHAL_CALLS,
        false,
    ));
    rows.push(FfiRow::new(
        "marshal",
        CASE_VEC,
        "Vec<u32> × 16 argument".into(),
        "[UInt32] of 16 serialized into a RustBuffer per call",
        MARSHAL_CALLS,
        false,
    ));
    rows
}

fn group_digits(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

impl Default for FfiBenchState {
    fn default() -> Self {
        Self {
            rows: default_rows(),
            running: false,
            steps_total: 0,
            steps_done: 0,
            current_label: None,
            last_total_ms: None,
        }
    }
}

impl FfiBenchState {
    pub fn row_mut(&mut self, id: &str) -> Option<&mut FfiRow> {
        self.rows.iter_mut().find(|r| r.id == id)
    }

    /// Prepare a run; `None` if already running.
    pub fn begin(&mut self) -> Option<Vec<String>> {
        if self.running {
            return None;
        }
        let mut steps = 0;
        for r in &mut self.rows {
            r.swift_inline = None;
            r.rust_per_call = None;
            r.rust_batched = None;
            r.finalize();
            steps += r.variants().len() as u32;
        }
        self.running = true;
        self.steps_total = steps;
        self.steps_done = 0;
        self.current_label = None;
        Some(self.rows.iter().map(|r| r.id.clone()).collect())
    }

    /// Median per-call overhead of the u64 sweep rows with ≥ 1,000 calls (ns).
    pub fn crossing_ns(&self) -> Option<f64> {
        let mut v: Vec<f64> = self
            .rows
            .iter()
            .filter(|r| r.case_id == CASE_U64 && r.calls >= 1_000)
            .filter_map(|r| r.overhead_ns)
            .collect();
        if v.is_empty() {
            return None;
        }
        v.sort_by(f64::total_cmp);
        Some(v[v.len() / 2])
    }

    /// Crossover analysis. `compute_speedup`: geometric-mean Rust/Swift ratio of the
    /// compute benchmarks, if measured.
    pub fn summary(&self, compute_speedup: Option<f64>) -> Option<String> {
        let c = self.crossing_ns()?;
        let s = compute_speedup.filter(|s| *s > 1.0);
        let inline = self
            .rows
            .iter()
            .filter(|r| r.case_id == CASE_U64 && r.calls >= 1_000)
            .filter_map(|r| r.swift_inline.as_ref().map(|x| x.ns_per_call))
            .fold(f64::NAN, f64::min);
        let batch1 = self
            .rows
            .iter()
            .find(|r| r.case_id == CASE_U64 && r.calls == 1)
            .and_then(|r| r.rust_batched.as_ref().map(|x| x.ns_per_call));
        let batch_big = self
            .rows
            .iter()
            .find(|r| r.case_id == CASE_U64 && r.calls == 100_000)
            .and_then(|r| r.batched_ratio);
        let mut out = format!("One Swift→Rust UniFFI call costs ≈ {c:.1} ns on top of the work");
        if inline.is_finite() {
            out += &format!(" (the tiny op itself: {inline:.2} ns inline in Swift)");
        }
        out += ". ";
        if let Some(b) = batch1 {
            out += &format!("A single batched call pays it once (≈ {b:.1} ns total). ");
        }
        if let Some(r) = batch_big {
            out += &format!(
                "At 100,000 ops per call the crossing is amortized: batched runs at {} . ",
                slowdown_text(r)
            );
        }
        match s {
            Some(s) => {
                let w = c * s / (s - 1.0);
                out += &format!(
                    "Break-even: with Rust {s:.2}× faster at compute (geomean of the compute tests), a call must carry ≥ {} of Swift-equivalent work before moving it to Rust pays off; below that, chatty calls make Rust slower.",
                    fmt_ns(w)
                );
            }
            None => {
                out += &format!(
                    "Break-even: if Rust is s× faster at the work, a call needs ≥ {c:.0}·s/(s−1) ns of Swift-equivalent work (e.g. {} at 2×). Run the compute tests to use the measured s.",
                    fmt_ns(c * 2.0)
                );
            }
        }
        Some(out)
    }

    pub fn view(&self, compute_speedup: Option<f64>) -> FfiSectionView {
        FfiSectionView {
            title: format!("#37 · {SECTION}: chatty calls"),
            intro: "The 36 compute tests time work inside each language; the FFI crossing happens once per test, outside the timer. Here Swift calls Rust once per tiny operation, so the crossing itself is what gets measured.".into(),
            rows: self.rows.clone(),
            running: self.running,
            progress: if self.steps_total == 0 {
                if self.rows.iter().any(|r| r.checksum_match.is_some()) { 1.0 } else { 0.0 }
            } else {
                (self.steps_done as f64 / self.steps_total as f64).min(1.0)
            },
            current_label: self.current_label.clone(),
            crossing_ns: self.crossing_ns(),
            summary: self.summary(compute_speedup),
            last_total_ms: self.last_total_ms,
        }
    }
}

fn fmt_ns(ns: f64) -> String {
    if ns >= 1000.0 {
        format!("{:.2} µs", ns / 1000.0)
    } else {
        format!("{ns:.0} ns")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_equals_chained_mix() {
        let mut a = 7;
        for i in 0..100 {
            a = mix(a, i);
        }
        assert_eq!(mix_batch(7, 100), a);
    }

    #[test]
    fn rows_have_reference_checksums() {
        let rows = default_rows();
        assert_eq!(rows.len(), SWEEP.len() + 3);
        assert!(rows.iter().all(|r| r.expected_checksum != 0));
        assert_eq!(rows[0].reps, TARGET_OPS);
        assert_eq!(rows[5].reps, 10);
    }

    #[test]
    fn summary_reports_break_even() {
        let mut st = FfiBenchState::default();
        for r in &mut st.rows {
            let ck = r.expected_checksum;
            r.swift_inline = Some(FfiSample {
                ns_per_call: 1.0,
                checksum: ck,
            });
            r.rust_per_call = Some(FfiSample {
                ns_per_call: 21.0,
                checksum: ck,
            });
            if r.has_batched {
                r.rust_batched = Some(FfiSample {
                    ns_per_call: 1.0,
                    checksum: ck,
                });
            }
            r.finalize();
            assert_eq!(r.checksum_match, Some(true));
        }
        assert_eq!(st.crossing_ns(), Some(20.0));
        let s = st.summary(Some(2.0)).unwrap();
        assert!(s.contains("40 ns"), "{s}");
    }
}
