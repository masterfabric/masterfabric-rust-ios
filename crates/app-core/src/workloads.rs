//! Benchmark workloads (Rust side).
//!
//! Every workload has a Swift twin written with the exact same algorithm
//! (`platforms/ios/MasterFabricRust/Benchmarks/SwiftWorkloads.swift`).
//! Both sides return a `checksum`; matching checksums prove both languages did the
//! same work. Parameters are passed at runtime from Rust so compilers cannot
//! constant-fold the results.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hint::black_box;
use std::time::Instant;

/// Benchmark ids. The Swift side uses the same ids.
pub const FIB: &str = "fib";
pub const SORT: &str = "sort";
pub const SHA256: &str = "sha256";
pub const JSON: &str = "json";
pub const SIEVE: &str = "sieve";
pub const MATMUL: &str = "matmul";
pub const HASHMAP: &str = "hashmap";
pub const RAYTRACE: &str = "raytrace";
pub const RASTER: &str = "raster";
pub const IMAGE: &str = "image";
pub const VIDEO: &str = "video";
pub const MESH: &str = "mesh";

/// xorshift32 generator, identical to the Swift version.
#[derive(Clone, Copy)]
pub struct XorShift32(pub u32);

impl XorShift32 {
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
}

/// One run. `None` for an unknown id.
pub fn run_once(id: &str, param: u64) -> Option<u64> {
    let param = black_box(param);
    let checksum = match id {
        FIB => fib(param as u32),
        SORT => sort_ints(param as usize),
        SHA256 => sha256_buffer(param as usize),
        JSON => json_roundtrip(param as usize),
        SIEVE => prime_sieve(param as usize),
        MATMUL => matmul(param as usize),
        HASHMAP => hashmap_count(param as usize),
        RAYTRACE => crate::media::raytrace_bench(param as usize),
        RASTER => crate::media::rasterize(param as usize),
        IMAGE => crate::media::image_pipeline(param as usize),
        VIDEO => crate::media::video_pipeline(param as usize),
        MESH => crate::media::mesh_bench(param as u32),
        _ => return None,
    };
    Some(black_box(checksum))
}

/// Multi-threaded (rayon) variant. `None` if the workload has no parallel version.
pub fn run_once_parallel(id: &str, param: u64) -> Option<u64> {
    let param = black_box(param);
    let checksum = match id {
        RAYTRACE => crate::media::raytrace_par_bench(param as usize),
        RASTER => crate::media::rasterize_par(param as usize),
        IMAGE => crate::media::image_pipeline_par(param as usize),
        VIDEO => crate::media::video_pipeline_par(param as usize),
        SORT => sort_ints_par(param as usize),
        MATMUL => matmul_par(param as usize),
        _ => return None,
    };
    Some(black_box(checksum))
}

/// 1 warm-up run + `iterations` timed runs. (avg ms, best ms, checksum)
pub fn measure(id: &str, param: u64, iterations: u32) -> Option<(f64, f64, u64)> {
    measure_with(|| run_once(id, param), iterations)
}

pub fn measure_parallel(id: &str, param: u64, iterations: u32) -> Option<(f64, f64, u64)> {
    measure_with(|| run_once_parallel(id, param), iterations)
}

fn measure_with(run: impl Fn() -> Option<u64>, iterations: u32) -> Option<(f64, f64, u64)> {
    let checksum = run()?; // warm-up
    let mut total = 0.0;
    let mut best = f64::MAX;
    for _ in 0..iterations.max(1) {
        let t = Instant::now();
        let c = run()?;
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        debug_assert_eq!(c, checksum);
        total += ms;
        best = best.min(ms);
    }
    Some((total / iterations.max(1) as f64, best, checksum))
}

// ---------------------------------------------------------------- workloads

/// Recursive fibonacci: function call overhead.
pub fn fib(n: u32) -> u64 {
    if n < 2 {
        n as u64
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

fn sort_input(n: usize) -> Vec<u32> {
    let mut rng = XorShift32(0x1234_5678);
    (0..n).map(|_| rng.next_u32()).collect()
}

/// Generate `n` pseudo-random u32 values and sort them.
pub fn sort_ints(n: usize) -> u64 {
    let mut v = sort_input(n);
    v.sort_unstable();
    sort_checksum(&v)
}

pub fn sort_ints_par(n: usize) -> u64 {
    let mut v = sort_input(n);
    v.par_sort_unstable();
    sort_checksum(&v)
}

fn sort_checksum(v: &[u32]) -> u64 {
    let n = v.len();
    let mut acc: u64 = 0;
    let mut i = 0;
    while i < n {
        acc = acc.wrapping_mul(31).wrapping_add(v[i] as u64);
        i += 1000;
    }
    acc ^ v[n - 1] as u64
}

/// SHA-256 of a deterministic `n`-byte buffer (hand-written, same in both languages).
pub fn sha256_buffer(n: usize) -> u64 {
    let data: Vec<u8> = (0..n as u32)
        .map(|i| i.wrapping_mul(31).wrapping_add(7) as u8)
        .collect();
    let d = sha256(&data);
    u64::from_be_bytes(d[..8].try_into().unwrap())
}

#[derive(Serialize, Deserialize)]
struct Record {
    id: u32,
    name: String,
    score: f64,
    active: bool,
    tags: Vec<String>,
}

/// Encode `n` records to JSON and decode them back (serde_json / Codable).
pub fn json_roundtrip(n: usize) -> u64 {
    let records: Vec<Record> = (0..n as u32)
        .map(|i| Record {
            id: i,
            name: format!("item-{i}"),
            score: i as f64 * 0.5,
            active: i % 2 == 0,
            tags: vec![
                "rust".to_string(),
                "swift".to_string(),
                format!("t{}", i % 10),
            ],
        })
        .collect();
    let text = serde_json::to_string(&records).expect("serialize");
    let parsed: Vec<Record> = serde_json::from_str(&text).expect("parse");
    parsed.iter().fold(0u64, |acc, r| {
        acc + r.id as u64 + r.active as u64 + r.tags.len() as u64 + r.score as u64
    })
}

/// Sieve of Eratosthenes: number of primes up to `n`.
pub fn prime_sieve(n: usize) -> u64 {
    let mut is_composite = vec![false; n + 1];
    let mut count = 0u64;
    let mut i = 2;
    while i <= n {
        if !is_composite[i] {
            count += 1;
            let mut j = i * i;
            while j <= n {
                is_composite[j] = true;
                j += i;
            }
        }
        i += 1;
    }
    count
}

fn matmul_inputs(n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut a = vec![0.0f64; n * n];
    let mut b = vec![0.0f64; n * n];
    for i in 0..n {
        for j in 0..n {
            a[i * n + j] = ((i * j) % 7) as f64 + 0.5;
            b[i * n + j] = ((i + j) % 5) as f64 - 1.0;
        }
    }
    (a, b)
}

/// One output row of C = A·B (i-k-j order; inner loop over contiguous slices vectorizes).
fn matmul_row(a: &[f64], b: &[f64], n: usize, i: usize, row: &mut [f64]) {
    for k in 0..n {
        let aik = a[i * n + k];
        let brow = &b[k * n..(k + 1) * n];
        for (c, &bv) in row.iter_mut().zip(brow) {
            *c += aik * bv;
        }
    }
}

/// `n x n` f64 matrix multiply (i-k-j loop order, same in both languages).
pub fn matmul(n: usize) -> u64 {
    let (a, b) = matmul_inputs(n);
    let mut c = vec![0.0f64; n * n];
    for (i, row) in c.chunks_exact_mut(n).enumerate() {
        matmul_row(&a, &b, n, i, row);
    }
    let sum: f64 = c.iter().sum();
    sum as i64 as u64
}

pub fn matmul_par(n: usize) -> u64 {
    let (a, b) = matmul_inputs(n);
    let mut c = vec![0.0f64; n * n];
    c.par_chunks_exact_mut(n)
        .enumerate()
        .for_each(|(i, row)| matmul_row(&a, &b, n, i, row));
    let sum: f64 = c.iter().sum();
    sum as i64 as u64
}

/// Count `n` keys: HashMap insert/update.
pub fn hashmap_count(n: usize) -> u64 {
    let mut rng = XorShift32(0xC0FF_EE11);
    let mut map: HashMap<u32, u32> = HashMap::new();
    for _ in 0..n {
        *map.entry(rng.next_u32() % 65_536).or_insert(0) += 1;
    }
    let max = map.values().copied().max().unwrap_or(0);
    ((map.len() as u64) << 32) | max as u64
}

// ---------------------------------------------------------------- SHA-256

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let (blocks, rem) = data.as_chunks::<64>();
    for block in blocks {
        compress(&mut h, block);
    }
    let mut tail = [0u8; 128];
    tail[..rem.len()].copy_from_slice(rem);
    tail[rem.len()] = 0x80;
    let total = if rem.len() < 56 { 64 } else { 128 };
    let bit_len = (data.len() as u64).wrapping_mul(8);
    tail[total - 8..total].copy_from_slice(&bit_len.to_be_bytes());
    for block in tail[..total].as_chunks::<64>().0 {
        compress(&mut h, block);
    }
    let mut out = [0u8; 32];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

fn compress(h: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 64];
    for i in 0..16 {
        w[i] = u32::from_be_bytes([
            block[i * 4],
            block[i * 4 + 1],
            block[i * 4 + 2],
            block[i * 4 + 3],
        ]);
    }
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = *h;
    for i in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ (!e & g);
        let t1 = hh
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(K[i])
            .wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        hh = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    for (slot, v) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
        *slot = slot.wrapping_add(v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        let long = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        assert_eq!(
            hex(&sha256(long)),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn workload_answers() {
        assert_eq!(fib(20), 6765);
        assert_eq!(prime_sieve(100), 25);
        assert_eq!(prime_sieve(5_000_000), 348_513);
        assert!(run_once("nope", 1).is_none());
    }
}
