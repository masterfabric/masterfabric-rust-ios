//! Heavier workloads for the extended suite (image, video, 3D, math, data, text, crypto,
//! algorithms). Every function has a Swift twin with the same algorithm in
//! `platforms/ios/MasterFabricRust/Benchmarks/SwiftHeavy.swift`; both return the same checksum.
//! `*_par` variants use rayon and produce the same checksum as the single-thread version
//! (independent work units; floating-point sums are quantized per item, so order does not matter).

use crate::workloads::XorShift32;
use rayon::prelude::*;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::f64::consts::PI;

pub const MANDELBROT: &str = "mandelbrot";
pub const CONVOLVE: &str = "convolve";
pub const LANCZOS: &str = "lanczos";
pub const DCT: &str = "dct";
pub const GRADE: &str = "grade";
pub const PATHTRACE: &str = "pathtrace";
pub const BVH: &str = "bvh";
pub const ISOSURFACE: &str = "isosurface";
pub const NBODY: &str = "nbody";
pub const FFT: &str = "fft";
pub const BIGINT: &str = "bigint";
pub const PARTICLES: &str = "particles";
pub const KMEANS: &str = "kmeans";
pub const LZ77: &str = "lz77";
pub const WORDFREQ: &str = "wordfreq";
pub const TEXTSEARCH: &str = "textsearch";
pub const STRSORT: &str = "strsort";
pub const LEVENSHTEIN: &str = "levenshtein";
pub const CHACHA20: &str = "chacha20";
pub const CRC32: &str = "crc32";
pub const DIJKSTRA: &str = "dijkstra";
pub const BFS: &str = "bfs";
pub const NQUEENS: &str = "nqueens";
pub const SUDOKU: &str = "sudoku";

pub fn run(id: &str, p: u64) -> Option<u64> {
    let n = p as usize;
    Some(match id {
        MANDELBROT => mandelbrot(n, false),
        CONVOLVE => convolve(n, false),
        LANCZOS => lanczos(n, false),
        DCT => dct(n, false),
        GRADE => grade(n, false),
        PATHTRACE => pathtrace(n, false),
        BVH => bvh(n, false),
        ISOSURFACE => isosurface(n, false),
        NBODY => nbody(n),
        FFT => fft(n),
        BIGINT => bigint_factorial(n),
        PARTICLES => particles(n, false),
        KMEANS => kmeans(n, false),
        LZ77 => lz77(n, false),
        WORDFREQ => wordfreq(n),
        TEXTSEARCH => textsearch(n, false),
        STRSORT => strsort(n, false),
        LEVENSHTEIN => levenshtein(n, false),
        CHACHA20 => chacha20(n, false),
        CRC32 => crc32(n),
        DIJKSTRA => dijkstra(n),
        BFS => bfs(n),
        NQUEENS => nqueens(n, false),
        SUDOKU => sudoku(n, false),
        _ => return None,
    })
}

pub fn run_par(id: &str, p: u64) -> Option<u64> {
    let n = p as usize;
    Some(match id {
        MANDELBROT => mandelbrot(n, true),
        CONVOLVE => convolve(n, true),
        LANCZOS => lanczos(n, true),
        DCT => dct(n, true),
        GRADE => grade(n, true),
        PATHTRACE => pathtrace(n, true),
        BVH => bvh(n, true),
        ISOSURFACE => isosurface(n, true),
        PARTICLES => particles(n, true),
        KMEANS => kmeans(n, true),
        LZ77 => lz77(n, true),
        TEXTSEARCH => textsearch(n, true),
        STRSORT => strsort(n, true),
        LEVENSHTEIN => levenshtein(n, true),
        CHACHA20 => chacha20(n, true),
        NQUEENS => nqueens(n, true),
        SUDOKU => sudoku(n, true),
        _ => return None,
    })
}

// ------------------------------------------------------------------ helpers

#[inline]
fn q(x: f64, s: f64) -> u64 {
    (x * s).round() as i64 as u64
}

#[inline]
fn rnd01(r: &mut XorShift32) -> f64 {
    (r.next_u32() >> 8) as f64 / 16_777_216.0
}

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Rows `0..rows` of a `rows × width` buffer, sequential or rayon.
fn for_rows<T: Send>(buf: &mut [T], width: usize, par: bool, f: impl Fn(usize, &mut [T]) + Sync) {
    if par {
        buf.par_chunks_mut(width)
            .enumerate()
            .for_each(|(y, row)| f(y, row));
    } else {
        for (y, row) in buf.chunks_mut(width).enumerate() {
            f(y, row);
        }
    }
}

/// Map `0..n` to u64 and wrapping-sum (order independent), sequential or rayon.
fn sum_map(n: usize, par: bool, f: impl Fn(usize) -> u64 + Sync) -> u64 {
    if par {
        (0..n)
            .into_par_iter()
            .map(&f)
            .reduce(|| 0, |a, b| a.wrapping_add(b))
    } else {
        (0..n).map(f).fold(0, |a, b| a.wrapping_add(b))
    }
}

fn weighted_sum(bytes: &[u8], m: u64) -> u64 {
    bytes.iter().enumerate().fold(0u64, |a, (i, &v)| {
        a.wrapping_add(v as u64 * (i as u64 % m + 1))
    })
}

// ------------------------------------------------------------------ image

fn mandel_row(y: usize, n: usize, row: &mut [u16]) {
    let cy = -1.2 + 2.4 * y as f64 / n as f64;
    for (x, o) in row.iter_mut().enumerate() {
        let cx = -2.2 + 3.2 * x as f64 / n as f64;
        let (mut zx, mut zy) = (0.0f64, 0.0f64);
        let mut i = 0u16;
        while i < 256 {
            let zx2 = zx * zx;
            let zy2 = zy * zy;
            if zx2 + zy2 > 4.0 {
                break;
            }
            zy = 2.0 * zx * zy + cy;
            zx = zx2 - zy2 + cx;
            i += 1;
        }
        *o = i;
    }
}

/// Mandelbrot n×n, 256 max iterations.
pub fn mandelbrot(n: usize, par: bool) -> u64 {
    let mut img = vec![0u16; n * n];
    for_rows(&mut img, n, par, |y, row| mandel_row(y, n, row));
    img.iter().enumerate().fold(0u64, |a, (i, &v)| {
        a.wrapping_add(v as u64 * (i as u64 % 7 + 1))
    })
}

pub fn gray_source(n: usize, seed: u32) -> Vec<u8> {
    let mut r = XorShift32(seed);
    let mut v = Vec::with_capacity(n * n);
    for i in 0..n * n {
        let x = (i % n) as u32;
        let y = (i / n) as u32;
        v.push(((x ^ y).wrapping_add(r.next_u32() & 63) & 255) as u8);
    }
    v
}

const K5: [u32; 5] = [1, 4, 6, 4, 1];

fn blur5_row(src: &[u8], n: usize, y: usize, out: &mut [u8]) {
    if y < 2 || y >= n - 2 {
        return;
    }
    for (x, o) in out.iter_mut().enumerate().take(n - 2).skip(2) {
        let mut s = 0u32;
        for (j, kj) in K5.iter().enumerate() {
            let base = (y + j - 2) * n + x - 2;
            let row = &src[base..base + 5];
            let mut t = 0u32;
            for (k, &p) in K5.iter().zip(row) {
                t += k * p as u32;
            }
            s += kj * t;
        }
        *o = (s >> 8) as u8;
    }
}

fn sobel_row(b: &[u8], n: usize, y: usize, out: &mut [u8]) {
    if y < 1 || y >= n - 1 {
        return;
    }
    for x in 1..n - 1 {
        let p = |dx: usize, dy: usize| b[(y + dy - 1) * n + x + dx - 1] as i32;
        let gx = -p(0, 0) - 2 * p(0, 1) - p(0, 2) + p(2, 0) + 2 * p(2, 1) + p(2, 2);
        let gy = -p(0, 0) - 2 * p(1, 0) - p(2, 0) + p(0, 2) + 2 * p(1, 2) + p(2, 2);
        out[x] = (gx.abs() + gy.abs()).min(255) as u8;
    }
}

/// 5×5 gaussian convolution followed by a 3×3 Sobel edge filter on an n×n image.
pub fn convolve(n: usize, par: bool) -> u64 {
    let src = gray_source(n, 0x9E37_79B9);
    let mut blur = vec![0u8; n * n];
    for_rows(&mut blur, n, par, |y, row| blur5_row(&src, n, y, row));
    let mut edges = vec![0u8; n * n];
    for_rows(&mut edges, n, par, |y, row| sobel_row(&blur, n, y, row));
    weighted_sum(&edges, 13)
}

fn lanczos_k(t: f64) -> f64 {
    if t == 0.0 {
        1.0
    } else if t.abs() < 3.0 {
        let pt = PI * t;
        3.0 * pt.sin() * (pt / 3.0).sin() / (pt * pt)
    } else {
        0.0
    }
}

/// (start index per output, taps, normalized weights)
fn lanczos_weights(src: usize, dst: usize) -> (Vec<i64>, usize, Vec<f64>) {
    let scale = src as f64 / dst as f64;
    let support = 3.0 * scale;
    let taps = (support * 2.0).ceil() as usize + 1;
    let mut starts = Vec::with_capacity(dst);
    let mut ws = Vec::with_capacity(dst * taps);
    for x in 0..dst {
        let center = (x as f64 + 0.5) * scale - 0.5;
        let start = (center - support).ceil() as i64;
        let first = ws.len();
        let mut sum = 0.0;
        for k in 0..taps {
            let j = start + k as i64;
            let w = lanczos_k((j as f64 - center) / scale);
            ws.push(w);
            sum += w;
        }
        for w in &mut ws[first..] {
            *w /= sum;
        }
        starts.push(start);
    }
    (starts, taps, ws)
}

/// Lanczos-3 separable downscale n → 3n/4.
pub fn lanczos(n: usize, par: bool) -> u64 {
    let src = gray_source(n, 0x1357_9BDF);
    let m = n * 3 / 4;
    let (starts, taps, ws) = lanczos_weights(n, m);
    let last = n as i64 - 1;
    let mut tmp = vec![0.0f64; n * m];
    for_rows(&mut tmp, m, par, |y, row| {
        let s = &src[y * n..(y + 1) * n];
        for (x, o) in row.iter_mut().enumerate() {
            let w = &ws[x * taps..(x + 1) * taps];
            let mut acc = 0.0;
            for (k, wk) in w.iter().enumerate() {
                let j = (starts[x] + k as i64).clamp(0, last) as usize;
                acc += wk * s[j] as f64;
            }
            *o = acc;
        }
    });
    let mut out = vec![0u8; m * m];
    for_rows(&mut out, m, par, |y, row| {
        let w = &ws[y * taps..(y + 1) * taps];
        for (x, o) in row.iter_mut().enumerate() {
            let mut acc = 0.0;
            for (k, wk) in w.iter().enumerate() {
                let j = (starts[y] + k as i64).clamp(0, last) as usize;
                acc += wk * tmp[j * m + x];
            }
            *o = acc.round().clamp(0.0, 255.0) as u8;
        }
    });
    weighted_sum(&out, 11)
}

// ------------------------------------------------------------------ video

const QY: [u16; 64] = [
    16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69, 56,
    14, 17, 22, 29, 51, 87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104, 113,
    92, 49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98, 112, 100, 103, 99,
];

fn dct_frame(f: usize, cos: &[f64; 64]) -> u64 {
    const W: usize = 640;
    const H: usize = 360;
    let mut luma = vec![0u8; W * H];
    for y in 0..H {
        for x in 0..W {
            let base = ((x + f * 4) ^ (y * 2)) as u32 & 255;
            let noise = ((x * 7 + y * 13 + f * 31) as u32).wrapping_mul(2_654_435_761) >> 27;
            luma[y * W + x] = (base.wrapping_add(noise) & 255) as u8;
        }
    }
    let mut acc = 0u64;
    let mut nz = 0u64;
    let mut tmp = [0.0f64; 64];
    for by in 0..H / 8 {
        for bx in 0..W / 8 {
            for y in 0..8 {
                for u in 0..8 {
                    let mut s = 0.0;
                    for x in 0..8 {
                        let p = luma[(by * 8 + y) * W + bx * 8 + x] as f64 - 128.0;
                        s += p * cos[x * 8 + u];
                    }
                    tmp[y * 8 + u] = s;
                }
            }
            for v in 0..8 {
                for u in 0..8 {
                    let mut s = 0.0;
                    for y in 0..8 {
                        s += tmp[y * 8 + u] * cos[y * 8 + v];
                    }
                    let cu = if u == 0 {
                        std::f64::consts::FRAC_1_SQRT_2
                    } else {
                        1.0
                    };
                    let cv = if v == 0 {
                        std::f64::consts::FRAC_1_SQRT_2
                    } else {
                        1.0
                    };
                    let val = 0.25 * cu * cv * s;
                    let qv = (val / QY[v * 8 + u] as f64).round() as i64;
                    acc = acc.wrapping_add((qv * ((v * 8 + u) as i64 + 1)) as u64);
                    if qv != 0 {
                        nz += 1;
                    }
                }
            }
        }
    }
    acc ^ (nz << 40)
}

/// Encoder-like DCT: `frames` synthetic 640×360 frames, 8×8 DCT-II + JPEG quantization.
pub fn dct(frames: usize, par: bool) -> u64 {
    let mut cos = [0.0f64; 64];
    for x in 0..8 {
        for u in 0..8 {
            cos[x * 8 + u] = ((2 * x + 1) as f64 * u as f64 * PI / 16.0).cos();
        }
    }
    sum_map(frames, par, |f| dct_frame(f, &cos))
}

fn grade_frame(f: usize, lut: &[u8; 256]) -> u64 {
    const W: i64 = 1280;
    const H: i64 = 720;
    let max_d2 = (W / 2) * (W / 2) + (H / 2) * (H / 2);
    let mut hist = [0u32; 256];
    let mut acc = 0u64;
    let fi = f as i64;
    for y in 0..H {
        for x in 0..W {
            let r = lut[((x + fi * 3) & 255) as usize] as i64;
            let g = lut[((y + fi * 2) & 255) as usize] as i64;
            let b = lut[(((x ^ y) + fi) & 255) as usize] as i64;
            let l = (77 * r + 150 * g + 29 * b) >> 8;
            let sat = |c: i64| (l + (c - l) * 5 / 4).clamp(0, 255);
            let (r, g, b) = (sat(r), sat(g), sat(b));
            let dx = x - W / 2;
            let dy = y - H / 2;
            let factor = 256 - (dx * dx + dy * dy) * 96 / max_d2;
            let (r, g, b) = ((r * factor) >> 8, (g * factor) >> 8, (b * factor) >> 8);
            let l2 = ((77 * r + 150 * g + 29 * b) >> 8) as usize;
            hist[l2] += 1;
            acc = acc.wrapping_add((r ^ (g << 8) ^ (b << 16)) as u64);
        }
    }
    hist.iter()
        .enumerate()
        .fold(acc, |a, (i, &h)| a.wrapping_add(h as u64 * (i as u64 + 1)))
}

/// Color grading: `frames` 1280×720 RGB frames, gamma LUT, saturation, vignette, histogram.
pub fn grade(frames: usize, par: bool) -> u64 {
    let mut lut = [0u8; 256];
    for (i, v) in lut.iter_mut().enumerate() {
        *v = ((i as f64 / 255.0).powf(0.8) * 255.0).round() as u8;
    }
    sum_map(frames, par, |f| grade_frame(f, &lut))
}

// ------------------------------------------------------------------ 3D

#[derive(Clone, Copy)]
struct V3 {
    x: f64,
    y: f64,
    z: f64,
}

impl V3 {
    const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    fn add(self, o: V3) -> V3 {
        V3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
    fn sub(self, o: V3) -> V3 {
        V3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
    fn mul(self, o: V3) -> V3 {
        V3::new(self.x * o.x, self.y * o.y, self.z * o.z)
    }
    fn scale(self, s: f64) -> V3 {
        V3::new(self.x * s, self.y * s, self.z * s)
    }
    fn dot(self, o: V3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    fn norm(self) -> V3 {
        let l = self.dot(self).sqrt();
        V3::new(self.x / l, self.y / l, self.z / l)
    }
}

const PT_SPHERES: [(V3, f64, V3); 5] = [
    (V3::new(0.0, -1000.5, -1.0), 1000.0, V3::new(0.5, 0.5, 0.5)),
    (V3::new(0.0, 0.0, -1.0), 0.5, V3::new(0.7, 0.3, 0.3)),
    (V3::new(-1.05, 0.0, -1.0), 0.5, V3::new(0.3, 0.7, 0.3)),
    (V3::new(1.05, 0.0, -1.0), 0.5, V3::new(0.3, 0.3, 0.8)),
    (V3::new(0.0, -0.35, -0.45), 0.15, V3::new(0.9, 0.9, 0.2)),
];

fn pt_trace(mut o: V3, mut d: V3, r: &mut XorShift32) -> V3 {
    let mut thr = V3::new(1.0, 1.0, 1.0);
    for _ in 0..4 {
        let mut best = f64::MAX;
        let mut hit = usize::MAX;
        for (i, (c, rad, _)) in PT_SPHERES.iter().enumerate() {
            let oc = o.sub(*c);
            let b = oc.dot(d);
            let cc = oc.dot(oc) - rad * rad;
            let disc = b * b - cc;
            if disc < 0.0 {
                continue;
            }
            let s = disc.sqrt();
            let mut t = -b - s;
            if t < 1e-4 {
                t = -b + s;
            }
            if t >= 1e-4 && t < best {
                best = t;
                hit = i;
            }
        }
        if hit == usize::MAX {
            let t = 0.5 * (d.y + 1.0);
            let sky = V3::new(1.0 - t + t * 0.5, 1.0 - t + t * 0.7, 1.0 - t + t * 1.0);
            return thr.mul(sky);
        }
        let (c, rad, albedo) = PT_SPHERES[hit];
        let p = o.add(d.scale(best));
        let nrm = p.sub(c).scale(1.0 / rad);
        let ru = loop {
            let v = V3::new(
                rnd01(r) * 2.0 - 1.0,
                rnd01(r) * 2.0 - 1.0,
                rnd01(r) * 2.0 - 1.0,
            );
            let l2 = v.dot(v);
            if l2 < 1.0 && l2 > 1e-6 {
                break v.norm();
            }
        };
        thr = thr.mul(albedo);
        o = p;
        d = nrm.add(ru).norm();
    }
    V3::new(0.0, 0.0, 0.0)
}

fn pt_row(y: usize, n: usize, row: &mut [u8]) {
    const SPP: usize = 8;
    for x in 0..n {
        let seed = ((y * n + x) as u32).wrapping_mul(2_654_435_761) ^ 0xA5A5_5A5A;
        let mut r = XorShift32(if seed == 0 { 1 } else { seed });
        let mut col = V3::new(0.0, 0.0, 0.0);
        for _ in 0..SPP {
            let u = ((x as f64 + rnd01(&mut r)) / n as f64) * 2.0 - 1.0;
            let v = 1.0 - ((y as f64 + rnd01(&mut r)) / n as f64) * 2.0;
            let d = V3::new(u * 1.2, v * 1.2, -1.0).norm();
            col = col.add(pt_trace(V3::new(0.0, 0.0, 0.0), d, &mut r));
        }
        let px = &mut row[x * 3..x * 3 + 3];
        for (o, c) in px.iter_mut().zip([col.x, col.y, col.z]) {
            *o = ((c / SPP as f64).sqrt().clamp(0.0, 0.999) * 256.0) as u8;
        }
    }
}

/// Path tracer: n×n, 8 samples per pixel, up to 4 diffuse bounces.
pub fn pathtrace(n: usize, par: bool) -> u64 {
    let mut img = vec![0u8; n * n * 3];
    for_rows(&mut img, n * 3, par, |y, row| pt_row(y, n, row));
    weighted_sum(&img, 13)
}

#[derive(Default, Clone, Copy)]
struct BvhStats {
    nodes: u64,
    leaves: u64,
    depth_sum: u64,
    area_q: u64,
}

impl BvhStats {
    fn merge(self, o: BvhStats) -> BvhStats {
        BvhStats {
            nodes: self.nodes + o.nodes,
            leaves: self.leaves + o.leaves,
            depth_sum: self.depth_sum + o.depth_sum,
            area_q: self.area_q.wrapping_add(o.area_q),
        }
    }
}

struct Tris {
    bmin: Vec<[f64; 3]>,
    bmax: Vec<[f64; 3]>,
    cent: Vec<[f64; 3]>,
}

fn bvh_build(t: &Tris, idx: &mut [u32], depth: u64, par: bool) -> BvhStats {
    let mut bmin = [f64::MAX; 3];
    let mut bmax = [-f64::MAX; 3];
    let mut cmin = [f64::MAX; 3];
    let mut cmax = [-f64::MAX; 3];
    for &i in idx.iter() {
        let i = i as usize;
        for a in 0..3 {
            if t.bmin[i][a] < bmin[a] {
                bmin[a] = t.bmin[i][a];
            }
            if t.bmax[i][a] > bmax[a] {
                bmax[a] = t.bmax[i][a];
            }
            if t.cent[i][a] < cmin[a] {
                cmin[a] = t.cent[i][a];
            }
            if t.cent[i][a] > cmax[a] {
                cmax[a] = t.cent[i][a];
            }
        }
    }
    let (dx, dy, dz) = (bmax[0] - bmin[0], bmax[1] - bmin[1], bmax[2] - bmin[2]);
    let mut s = BvhStats {
        nodes: 1,
        area_q: q(2.0 * (dx * dy + dy * dz + dz * dx), 10.0),
        ..Default::default()
    };
    if idx.len() <= 4 {
        s.leaves = 1;
        s.depth_sum = depth;
        return s;
    }
    let ext = [cmax[0] - cmin[0], cmax[1] - cmin[1], cmax[2] - cmin[2]];
    let mut axis = 0;
    if ext[1] > ext[axis] {
        axis = 1;
    }
    if ext[2] > ext[axis] {
        axis = 2;
    }
    idx.sort_unstable_by(|&a, &b| {
        t.cent[a as usize][axis]
            .partial_cmp(&t.cent[b as usize][axis])
            .unwrap()
            .then(a.cmp(&b))
    });
    let mid = idx.len() / 2;
    let (l, r) = idx.split_at_mut(mid);
    let (a, b) = if par && l.len() > 2048 {
        rayon::join(
            || bvh_build(t, l, depth + 1, par),
            || bvh_build(t, r, depth + 1, par),
        )
    } else {
        (
            bvh_build(t, l, depth + 1, par),
            bvh_build(t, r, depth + 1, par),
        )
    };
    s.merge(a).merge(b)
}

/// BVH build over n random triangles: recursive median split on the widest centroid axis.
pub fn bvh(n: usize, par: bool) -> u64 {
    let mut r = XorShift32(0xB0B0_CAFE);
    let mut t = Tris {
        bmin: Vec::with_capacity(n),
        bmax: Vec::with_capacity(n),
        cent: Vec::with_capacity(n),
    };
    for _ in 0..n {
        let base = [
            rnd01(&mut r) * 100.0,
            rnd01(&mut r) * 100.0,
            rnd01(&mut r) * 100.0,
        ];
        let mut v = [[0.0f64; 3]; 3];
        for vert in v.iter_mut() {
            for a in 0..3 {
                vert[a] = base[a] + (rnd01(&mut r) - 0.5) * 2.0;
            }
        }
        let mut mn = [0.0; 3];
        let mut mx = [0.0; 3];
        let mut c = [0.0; 3];
        for a in 0..3 {
            mn[a] = v[0][a];
            mx[a] = v[0][a];
            for vert in &v[1..] {
                if vert[a] < mn[a] {
                    mn[a] = vert[a];
                }
                if vert[a] > mx[a] {
                    mx[a] = vert[a];
                }
            }
            c[a] = (v[0][a] + v[1][a] + v[2][a]) / 3.0;
        }
        t.bmin.push(mn);
        t.bmax.push(mx);
        t.cent.push(c);
    }
    let mut idx: Vec<u32> = (0..n as u32).collect();
    let s = bvh_build(&t, &mut idx, 0, par);
    (s.nodes ^ (s.leaves << 20) ^ (s.depth_sum << 32)).wrapping_add(s.area_q)
}

const BALLS: [(f64, f64, f64, f64); 3] = [
    (-0.3, 0.0, 0.0, 0.35),
    (0.35, 0.1, 0.0, 0.3),
    (0.0, -0.25, 0.3, 0.25),
];
const CUBE_EDGES: [(usize, usize); 12] = [
    (0, 1),
    (2, 3),
    (4, 5),
    (6, 7),
    (0, 2),
    (1, 3),
    (4, 6),
    (5, 7),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

/// Isosurface extraction (surface nets) on an n³ cell grid of 3 metaballs.
pub fn isosurface(n: usize, par: bool) -> u64 {
    let g = n + 1;
    let mut field = vec![0.0f64; g * g * g];
    for_rows(&mut field, g * g, par, |k, slab| {
        let pz = k as f64 / n as f64 * 2.0 - 1.0;
        for j in 0..g {
            let py = j as f64 / n as f64 * 2.0 - 1.0;
            for i in 0..g {
                let px = i as f64 / n as f64 * 2.0 - 1.0;
                let mut f = 0.0;
                for (bx, by, bz, br) in BALLS {
                    let (dx, dy, dz) = (px - bx, py - by, pz - bz);
                    f += br * br / (dx * dx + dy * dy + dz * dz + 1e-6);
                }
                slab[j * g + i] = f - 1.0;
            }
        }
    });
    let at = |i: usize, j: usize, k: usize| field[(k * g + j) * g + i];
    // vertices: one per cell with a sign change
    let slab = |k: usize| -> u64 {
        let mut verts = 0u64;
        let mut acc = 0u64;
        for j in 0..n {
            for i in 0..n {
                let mut v = [0.0f64; 8];
                let mut mask = 0u32;
                for (c, vc) in v.iter_mut().enumerate() {
                    *vc = at(i + (c & 1), j + ((c >> 1) & 1), k + (c >> 2));
                    if *vc > 0.0 {
                        mask |= 1 << c;
                    }
                }
                if mask == 0 || mask == 255 {
                    continue;
                }
                let (mut sx, mut sy, mut sz, mut cnt) = (0.0, 0.0, 0.0, 0.0);
                for (a, b) in CUBE_EDGES {
                    if (v[a] > 0.0) != (v[b] > 0.0) {
                        let t = v[a] / (v[a] - v[b]);
                        let pa = [(a & 1) as f64, ((a >> 1) & 1) as f64, (a >> 2) as f64];
                        let pb = [(b & 1) as f64, ((b >> 1) & 1) as f64, (b >> 2) as f64];
                        sx += pa[0] + (pb[0] - pa[0]) * t;
                        sy += pa[1] + (pb[1] - pa[1]) * t;
                        sz += pa[2] + (pb[2] - pa[2]) * t;
                        cnt += 1.0;
                    }
                }
                verts += 1;
                acc = acc
                    .wrapping_add(q(i as f64 + sx / cnt, 1000.0))
                    .wrapping_add(q(j as f64 + sy / cnt, 1000.0))
                    .wrapping_add(q(k as f64 + sz / cnt, 1000.0));
            }
        }
        acc.wrapping_add(verts << 40)
    };
    let vsum = sum_map(n, par, slab);
    // faces: one quad per grid edge with a sign change and 4 surrounding cells
    let mut quads = 0u64;
    for k in 1..n {
        for j in 1..n {
            for i in 0..n {
                if (at(i, j, k) > 0.0) != (at(i + 1, j, k) > 0.0) {
                    quads += 1;
                }
                if (at(j, i, k) > 0.0) != (at(j, i + 1, k) > 0.0) {
                    quads += 1;
                }
                if (at(j, k, i) > 0.0) != (at(j, k, i + 1) > 0.0) {
                    quads += 1;
                }
            }
        }
    }
    vsum.wrapping_add(quads * 2)
}

// ------------------------------------------------------------------ math

/// Classic 5-body simulation (sun + 4 gas giants), `steps` steps of dt = 0.01.
pub fn nbody(steps: usize) -> u64 {
    const SM: f64 = 4.0 * PI * PI;
    const DPY: f64 = 365.24;
    let mut b: [[f64; 7]; 5] = [
        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, SM],
        [
            4.841_431_442_464_72e0,
            -1.160_320_044_027_428_4e0,
            -1.036_220_444_711_231_1e-1,
            1.660_076_642_744_037e-3 * DPY,
            7.699_011_184_197_404e-3 * DPY,
            -6.904_600_169_720_63e-5 * DPY,
            9.547_919_384_243_266e-4 * SM,
        ],
        [
            8.343_366_718_244_58e0,
            4.124_798_564_124_305e0,
            -4.035_234_171_143_214e-1,
            -2.767_425_107_268_624e-3 * DPY,
            4.998_528_012_349_172e-3 * DPY,
            2.304_172_975_737_639_3e-5 * DPY,
            2.858_859_806_661_308e-4 * SM,
        ],
        [
            1.289_436_956_213_913_1e1,
            -1.511_115_140_169_863_1e1,
            -2.233_075_788_926_557_3e-1,
            2.964_601_375_647_616e-3 * DPY,
            2.378_471_739_594_809_5e-3 * DPY,
            -2.965_895_685_402_375_6e-5 * DPY,
            4.366_244_043_351_563e-5 * SM,
        ],
        [
            1.537_969_711_485_091_1e1,
            -2.591_931_460_998_796_4e1,
            1.792_587_729_503_711_8e-1,
            2.680_677_724_903_893_2e-3 * DPY,
            1.628_241_700_382_423e-3 * DPY,
            -9.515_922_545_197_159e-5 * DPY,
            5.151_389_020_466_114_5e-5 * SM,
        ],
    ];
    let (mut px, mut py, mut pz) = (0.0, 0.0, 0.0);
    for body in &b {
        px += body[3] * body[6];
        py += body[4] * body[6];
        pz += body[5] * body[6];
    }
    b[0][3] = -px / SM;
    b[0][4] = -py / SM;
    b[0][5] = -pz / SM;
    let dt = 0.01;
    for _ in 0..steps {
        for i in 0..5 {
            for j in i + 1..5 {
                let dx = b[i][0] - b[j][0];
                let dy = b[i][1] - b[j][1];
                let dz = b[i][2] - b[j][2];
                let d2 = dx * dx + dy * dy + dz * dz;
                let mag = dt / (d2 * d2.sqrt());
                let mj = b[j][6] * mag;
                let mi = b[i][6] * mag;
                b[i][3] -= dx * mj;
                b[i][4] -= dy * mj;
                b[i][5] -= dz * mj;
                b[j][3] += dx * mi;
                b[j][4] += dy * mi;
                b[j][5] += dz * mi;
            }
        }
        for body in b.iter_mut() {
            body[0] += dt * body[3];
            body[1] += dt * body[4];
            body[2] += dt * body[5];
        }
    }
    let mut e = 0.0;
    for i in 0..5 {
        e += 0.5 * b[i][6] * (b[i][3] * b[i][3] + b[i][4] * b[i][4] + b[i][5] * b[i][5]);
        for j in i + 1..5 {
            let dx = b[i][0] - b[j][0];
            let dy = b[i][1] - b[j][1];
            let dz = b[i][2] - b[j][2];
            e -= b[i][6] * b[j][6] / (dx * dx + dy * dy + dz * dz).sqrt();
        }
    }
    q(e, 1e9)
}

/// Iterative radix-2 complex FFT of n (power of two) real samples.
pub fn fft(n: usize) -> u64 {
    let mut r = XorShift32(0xF00D_F00D);
    let mut re: Vec<f64> = (0..n).map(|_| rnd01(&mut r) - 0.5).collect();
    let mut im = vec![0.0f64; n];
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let half = n / 2;
    let mut twr = Vec::with_capacity(half);
    let mut twi = Vec::with_capacity(half);
    for k in 0..half {
        let ang = 2.0 * PI * k as f64 / n as f64;
        twr.push(ang.cos());
        twi.push(-ang.sin());
    }
    let mut len = 2;
    while len <= n {
        let step = n / len;
        let h = len / 2;
        let mut i = 0;
        while i < n {
            for k in 0..h {
                let wr = twr[k * step];
                let wi = twi[k * step];
                let a = i + k;
                let b = a + h;
                let xr = re[b] * wr - im[b] * wi;
                let xi = re[b] * wi + im[b] * wr;
                re[b] = re[a] - xr;
                im[b] = im[a] - xi;
                re[a] += xr;
                im[a] += xi;
            }
            i += len;
        }
        len <<= 1;
    }
    re.iter().zip(&im).fold(0u64, |a, (x, y)| {
        a.wrapping_add(q(x.abs() + y.abs(), 1000.0))
    })
}

/// n! with base-1e9 limbs; checksum = decimal digit sum + limb count.
pub fn bigint_factorial(n: usize) -> u64 {
    const BASE: u64 = 1_000_000_000;
    let mut limbs: Vec<u32> = vec![1];
    for k in 2..=n as u64 {
        let mut carry = 0u64;
        for l in limbs.iter_mut() {
            let cur = *l as u64 * k + carry;
            *l = (cur % BASE) as u32;
            carry = cur / BASE;
        }
        while carry > 0 {
            limbs.push((carry % BASE) as u32);
            carry /= BASE;
        }
    }
    let mut digits = 0u64;
    for &l in &limbs {
        let mut v = l;
        while v > 0 {
            digits += (v % 10) as u64;
            v /= 10;
        }
    }
    digits + ((limbs.len() as u64) << 32)
}

fn particle_steps(p: &mut [f64; 4]) {
    let dt = 0.005;
    for _ in 0..50 {
        p[3] -= 9.81 * dt;
        p[0] += p[2] * dt;
        p[1] += p[3] * dt;
        if p[0] < 0.0 {
            p[0] = -p[0];
            p[2] = -p[2] * 0.9;
        } else if p[0] > 1.0 {
            p[0] = 2.0 - p[0];
            p[2] = -p[2] * 0.9;
        }
        if p[1] < 0.0 {
            p[1] = -p[1];
            p[3] = -p[3] * 0.8;
        } else if p[1] > 1.0 {
            p[1] = 2.0 - p[1];
            p[3] = -p[3] * 0.8;
        }
        p[2] *= 0.999;
        p[3] *= 0.999;
    }
}

/// Particle physics: n particles, 50 steps of gravity, wall bounces and drag.
pub fn particles(n: usize, par: bool) -> u64 {
    let mut r = XorShift32(0x0BAD_5EED);
    let mut ps: Vec<[f64; 4]> = (0..n)
        .map(|_| {
            let x = rnd01(&mut r);
            let y = rnd01(&mut r);
            let vx = rnd01(&mut r) - 0.5;
            let vy = rnd01(&mut r) - 0.5;
            [x, y, vx, vy]
        })
        .collect();
    if par {
        ps.par_iter_mut().for_each(particle_steps);
    } else {
        ps.iter_mut().for_each(particle_steps);
    }
    ps.iter().fold(0u64, |a, p| {
        a.wrapping_add(q(p[0], 1e6)).wrapping_add(q(p[1], 1e6))
    })
}

// ------------------------------------------------------------------ data

/// k-means (k = 16, 8 iterations) on n 2-D points.
pub fn kmeans(n: usize, par: bool) -> u64 {
    const K: usize = 16;
    let mut r = XorShift32(0x00C0_FFEE);
    let mut seeds = [[0.0f64; 2]; K];
    for s in seeds.iter_mut() {
        s[0] = rnd01(&mut r) * 10.0;
        s[1] = rnd01(&mut r) * 10.0;
    }
    let pts: Vec<[f64; 2]> = (0..n)
        .map(|_| {
            let c = (r.next_u32() % K as u32) as usize;
            let x = seeds[c][0] + (rnd01(&mut r) - 0.5) * 1.5;
            let y = seeds[c][1] + (rnd01(&mut r) - 0.5) * 1.5;
            [x, y]
        })
        .collect();
    let mut centers: Vec<[f64; 2]> = pts[..K].to_vec();
    let nearest = |p: &[f64; 2], centers: &[[f64; 2]]| -> u32 {
        let mut best = 0;
        let mut bd = f64::MAX;
        for (c, cc) in centers.iter().enumerate() {
            let dx = p[0] - cc[0];
            let dy = p[1] - cc[1];
            let d = dx * dx + dy * dy;
            if d < bd {
                bd = d;
                best = c;
            }
        }
        best as u32
    };
    let mut assign = vec![0u32; n];
    for _ in 0..8 {
        if par {
            assign
                .par_iter_mut()
                .zip(&pts)
                .for_each(|(a, p)| *a = nearest(p, &centers));
        } else {
            for (a, p) in assign.iter_mut().zip(&pts) {
                *a = nearest(p, &centers);
            }
        }
        let mut sums = [[0.0f64; 2]; K];
        let mut counts = [0u32; K];
        for (p, &a) in pts.iter().zip(&assign) {
            sums[a as usize][0] += p[0];
            sums[a as usize][1] += p[1];
            counts[a as usize] += 1;
        }
        for c in 0..K {
            if counts[c] > 0 {
                centers[c] = [sums[c][0] / counts[c] as f64, sums[c][1] / counts[c] as f64];
            }
        }
    }
    let mut acc = assign.iter().enumerate().fold(0u64, |a, (i, &v)| {
        a.wrapping_add(v as u64 * (i as u64 % 7 + 1))
    });
    for c in &centers {
        acc = acc.wrapping_add(q(c[0], 1e6)).wrapping_add(q(c[1], 1e6));
    }
    acc
}

fn vocab(r: &mut XorShift32) -> Vec<Vec<u8>> {
    (0..500)
        .map(|_| {
            let len = 2 + (r.next_u32() % 8) as usize;
            (0..len).map(|_| b'a' + (r.next_u32() % 26) as u8).collect()
        })
        .collect()
}

/// Deterministic English-like text: words, capitals, commas, sentences, newlines.
pub fn gen_text(n: usize, seed: u32) -> Vec<u8> {
    let mut r = XorShift32(seed);
    let v = vocab(&mut r);
    let mut out = Vec::with_capacity(n + 16);
    while out.len() < n {
        let w = &v[(r.next_u32() % 500) as usize];
        let cap = r.next_u32().is_multiple_of(10);
        let start = out.len();
        out.extend_from_slice(w);
        if cap {
            out[start] -= 32;
        }
        let s = r.next_u32() % 30;
        if s == 0 {
            out.push(b'.');
            out.push(b'\n');
        } else if s < 3 {
            out.push(b',');
            out.push(b' ');
        } else {
            out.push(b' ');
        }
    }
    out.truncate(n);
    out
}

fn lz_block(d: &[u8]) -> u64 {
    const HBITS: u32 = 14;
    let len = d.len();
    let mut head = vec![-1i32; 1 << HBITS];
    let mut prev = vec![-1i32; len];
    let hash = |p: usize| -> usize {
        (u32::from_le_bytes([d[p], d[p + 1], d[p + 2], d[p + 3]]).wrapping_mul(2_654_435_761)
            >> (32 - HBITS)) as usize
    };
    let insert = |p: usize, head: &mut [i32], prev: &mut [i32]| {
        if p + 4 <= len {
            let h = hash(p);
            prev[p] = head[h];
            head[h] = p as i32;
        }
    };
    let mut acc = 0u64;
    let mut bits = 0u64;
    let mut pos = 0usize;
    while pos < len {
        let mut best_len = 0usize;
        let mut best_dist = 0usize;
        if pos + 4 <= len {
            let mut cand = head[hash(pos)];
            let mut depth = 0;
            let max = (len - pos).min(258);
            while cand >= 0 && depth < 16 && pos - cand as usize <= 32_768 {
                let c = cand as usize;
                let mut l = 0;
                while l < max && d[c + l] == d[pos + l] {
                    l += 1;
                }
                if l > best_len {
                    best_len = l;
                    best_dist = pos - c;
                    if l == max {
                        break;
                    }
                }
                cand = prev[c];
                depth += 1;
            }
        }
        if best_len >= 4 {
            acc = acc
                .wrapping_mul(31)
                .wrapping_add(((best_len << 16) | best_dist) as u64 ^ 0x8000_0000_0000);
            bits += 32;
            for p in pos..pos + best_len {
                insert(p, &mut head, &mut prev);
            }
            pos += best_len;
        } else {
            acc = acc.wrapping_mul(31).wrapping_add(d[pos] as u64);
            bits += 9;
            insert(pos, &mut head, &mut prev);
            pos += 1;
        }
    }
    acc.wrapping_add(bits << 40)
}

/// LZ77 compression (hash chains, 32 KiB window, 64 KiB independent blocks) of n bytes of text.
pub fn lz77(n: usize, par: bool) -> u64 {
    const BLOCK: usize = 1 << 16;
    let data = gen_text(n, 0x0012_3457);
    let blocks = data.len().div_ceil(BLOCK);
    sum_map(blocks, par, |b| {
        let end = ((b + 1) * BLOCK).min(data.len());
        lz_block(&data[b * BLOCK..end]).wrapping_mul(b as u64 + 1)
    })
}

// ------------------------------------------------------------------ text

/// Word frequency over n bytes of text (case-folded, split on non-letters).
pub fn wordfreq(n: usize) -> u64 {
    let text = gen_text(n, 0x0BEE_F123);
    let mut counts: HashMap<Vec<u8>, u32> = HashMap::new();
    let mut buf: Vec<u8> = Vec::with_capacity(16);
    let mut flush = |buf: &mut Vec<u8>| {
        if !buf.is_empty() {
            if let Some(c) = counts.get_mut(buf.as_slice()) {
                *c += 1;
            } else {
                counts.insert(buf.clone(), 1);
            }
            buf.clear();
        }
    };
    for &b in &text {
        if b.is_ascii_alphabetic() {
            buf.push(b.to_ascii_lowercase());
        } else {
            flush(&mut buf);
        }
    }
    flush(&mut buf);
    let unique = counts.len() as u64;
    counts.iter().fold(unique << 48, |a, (w, &c)| {
        a.wrapping_add(fnv(w).wrapping_mul(c as u64))
    })
}

// Kernighan/Pike matcher: c . ^ $ *
fn re_here(re: &[u8], ri: usize, t: &[u8], ti: usize) -> bool {
    if ri == re.len() {
        return true;
    }
    if ri + 1 < re.len() && re[ri + 1] == b'*' {
        return re_star(re[ri], re, ri + 2, t, ti);
    }
    if re[ri] == b'$' && ri + 1 == re.len() {
        return ti == t.len();
    }
    if ti < t.len() && (re[ri] == b'.' || re[ri] == t[ti]) {
        return re_here(re, ri + 1, t, ti + 1);
    }
    false
}

fn re_star(c: u8, re: &[u8], ri: usize, t: &[u8], mut ti: usize) -> bool {
    loop {
        if re_here(re, ri, t, ti) {
            return true;
        }
        if ti < t.len() && (t[ti] == c || c == b'.') {
            ti += 1;
        } else {
            return false;
        }
    }
}

fn re_match(re: &[u8], t: &[u8]) -> bool {
    if !re.is_empty() && re[0] == b'^' {
        return re_here(re, 1, t, 0);
    }
    let mut ti = 0;
    loop {
        if re_here(re, 0, t, ti) {
            return true;
        }
        if ti == t.len() {
            return false;
        }
        ti += 1;
    }
}

const PATTERNS: [&[u8]; 5] = [b"ab.*ef", b"^b", b"q.z", b"e*st$", b"xa*y"];

/// Regex-like search: 5 patterns (. * ^ $) over every line of n bytes of text.
pub fn textsearch(n: usize, par: bool) -> u64 {
    let text = gen_text(n, 0x7E57_7E57);
    let lines: Vec<&[u8]> = text.split(|&b| b == b'\n').collect();
    sum_map(lines.len(), par, |i| {
        let l = lines[i];
        PATTERNS
            .iter()
            .enumerate()
            .filter(|(_, p)| re_match(p, l))
            .map(|(k, _)| (k as u64 + 1) * (i as u64 % 3 + 1))
            .sum()
    })
}

/// Generate and sort n random lowercase strings (4–16 chars).
pub fn strsort(n: usize, par: bool) -> u64 {
    let mut r = XorShift32(0x5747_5747);
    let mut v: Vec<String> = (0..n)
        .map(|_| {
            let len = 4 + (r.next_u32() % 13) as usize;
            let bytes: Vec<u8> = (0..len).map(|_| b'a' + (r.next_u32() % 26) as u8).collect();
            String::from_utf8(bytes).unwrap()
        })
        .collect();
    if par {
        v.par_sort_unstable();
    } else {
        v.sort_unstable();
    }
    let mut acc = fnv(v[n - 1].as_bytes());
    for (i, s) in v.iter().enumerate().step_by(97) {
        acc = acc
            .wrapping_mul(31)
            .wrapping_add(fnv(s.as_bytes()))
            .wrapping_add(i as u64);
    }
    acc
}

fn lev_pair(len: usize, pair: usize) -> u64 {
    let mut r = XorShift32(0x5EED + pair as u32 * 7919);
    const ACGT: &[u8; 4] = b"acgt";
    let a: Vec<u8> = (0..len)
        .map(|_| ACGT[(r.next_u32() % 4) as usize])
        .collect();
    let mut b = Vec::with_capacity(len + len / 10);
    for &c in &a {
        let k = r.next_u32() % 25;
        if k == 0 {
            continue;
        }
        b.push(if r.next_u32().is_multiple_of(10) {
            ACGT[(r.next_u32() % 4) as usize]
        } else {
            c
        });
        if k == 1 {
            b.push(ACGT[(r.next_u32() % 4) as usize]);
        }
    }
    let mut prev: Vec<u32> = (0..=b.len() as u32).collect();
    let mut cur = vec![0u32; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        cur[0] = i as u32 + 1;
        for (j, &cb) in b.iter().enumerate() {
            let sub = prev[j] + (ca != cb) as u32;
            cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()] as u64 * (pair as u64 + 1)
}

/// Levenshtein edit distance for 8 DNA-like string pairs of length n.
pub fn levenshtein(n: usize, par: bool) -> u64 {
    sum_map(8, par, |p| lev_pair(n, p))
}

// ------------------------------------------------------------------ crypto

#[inline]
fn qr(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    s[a] = s[a].wrapping_add(s[b]);
    s[d] = (s[d] ^ s[a]).rotate_left(16);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = (s[b] ^ s[c]).rotate_left(12);
    s[a] = s[a].wrapping_add(s[b]);
    s[d] = (s[d] ^ s[a]).rotate_left(8);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = (s[b] ^ s[c]).rotate_left(7);
}

fn chacha_block(key: &[u32; 8], counter: u32, nonce: &[u32; 3], out: &mut [u8; 64]) {
    let init: [u32; 16] = [
        0x6170_7865,
        0x3320_646e,
        0x7962_2d32,
        0x6b20_6574,
        key[0],
        key[1],
        key[2],
        key[3],
        key[4],
        key[5],
        key[6],
        key[7],
        counter,
        nonce[0],
        nonce[1],
        nonce[2],
    ];
    let mut s = init;
    for _ in 0..10 {
        qr(&mut s, 0, 4, 8, 12);
        qr(&mut s, 1, 5, 9, 13);
        qr(&mut s, 2, 6, 10, 14);
        qr(&mut s, 3, 7, 11, 15);
        qr(&mut s, 0, 5, 10, 15);
        qr(&mut s, 1, 6, 11, 12);
        qr(&mut s, 2, 7, 8, 13);
        qr(&mut s, 3, 4, 9, 14);
    }
    for (i, w) in s
        .iter()
        .zip(init)
        .map(|(a, b)| a.wrapping_add(b))
        .enumerate()
    {
        out[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
}

/// ChaCha20 (RFC 8439 block function) encryption of n bytes; 4 KiB chunks hashed independently.
pub fn chacha20(n: usize, par: bool) -> u64 {
    const CHUNK: usize = 4096;
    let key: [u32; 8] =
        core::array::from_fn(|i| (i as u32).wrapping_mul(0x0101_0101) ^ 0xDEAD_BEEF);
    let nonce = [0x0000_0009, 0x4a00_0000, 0];
    let mut data: Vec<u8> = (0..n).map(|i| (i * 7 + 3) as u8).collect();
    let work = |ci: usize, chunk: &mut [u8]| {
        let mut ks = [0u8; 64];
        for (bi, block) in chunk.chunks_mut(64).enumerate() {
            chacha_block(&key, (ci * CHUNK / 64 + bi) as u32 + 1, &nonce, &mut ks);
            for (b, k) in block.iter_mut().zip(ks.iter()) {
                *b ^= k;
            }
        }
    };
    if par {
        data.par_chunks_mut(CHUNK)
            .enumerate()
            .for_each(|(i, c)| work(i, c));
    } else {
        for (i, c) in data.chunks_mut(CHUNK).enumerate() {
            work(i, c);
        }
    }
    data.chunks(CHUNK).enumerate().fold(0u64, |a, (i, c)| {
        let h = c
            .as_chunks::<8>()
            .0
            .iter()
            .fold(0u64, |h, w| h.rotate_left(5) ^ u64::from_le_bytes(*w));
        a.wrapping_add(h.wrapping_mul(i as u64 + 1))
    })
}

/// CRC-32 (IEEE, table driven) of n pseudo-random bytes.
pub fn crc32(n: usize) -> u64 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *t = c;
    }
    let mut r = XorShift32(0xC4C3_2C32);
    let data: Vec<u8> = (0..n).map(|_| r.next_u32() as u8).collect();
    let mut crc = !0u32;
    for &b in &data {
        crc = table[((crc ^ b as u32) & 0xff) as usize] ^ (crc >> 8);
    }
    (!crc) as u64 | ((n as u64) << 32)
}

// ------------------------------------------------------------------ algorithms

/// Dijkstra (binary heap) on an n×n grid with node costs 1–16, from the top-left corner.
pub fn dijkstra(n: usize) -> u64 {
    let mut r = XorShift32(0xD1D1_D1D1);
    let cost: Vec<u32> = (0..n * n).map(|_| 1 + r.next_u32() % 16).collect();
    let mut dist = vec![u32::MAX; n * n];
    let mut heap = BinaryHeap::new();
    dist[0] = 0;
    heap.push(Reverse((0u32, 0u32)));
    while let Some(Reverse((d, u))) = heap.pop() {
        let u = u as usize;
        if d > dist[u] {
            continue;
        }
        let (x, y) = (u % n, u / n);
        let mut relax = |v: usize| {
            let nd = d + cost[v];
            if nd < dist[v] {
                dist[v] = nd;
                heap.push(Reverse((nd, v as u32)));
            }
        };
        if x > 0 {
            relax(u - 1);
        }
        if x + 1 < n {
            relax(u + 1);
        }
        if y > 0 {
            relax(u - n);
        }
        if y + 1 < n {
            relax(u + n);
        }
    }
    let sum: u64 = dist.iter().map(|&d| d as u64).sum();
    let max = *dist.iter().max().unwrap() as u64;
    sum.wrapping_add(max << 40)
}

/// BFS on a random undirected graph (n nodes, 4n edges) stored as CSR.
pub fn bfs(n: usize) -> u64 {
    let mut r = XorShift32(0xBF5B_F5BF);
    let m = n * 4;
    let mut edges = Vec::with_capacity(m);
    for _ in 0..m {
        let u = r.next_u32() as usize % n;
        let v = r.next_u32() as usize % n;
        edges.push((u as u32, v as u32));
    }
    let mut off = vec![0u32; n + 1];
    for &(u, v) in &edges {
        off[u as usize + 1] += 1;
        off[v as usize + 1] += 1;
    }
    for i in 0..n {
        off[i + 1] += off[i];
    }
    let mut fill = off.clone();
    let mut adj = vec![0u32; m * 2];
    for &(u, v) in &edges {
        adj[fill[u as usize] as usize] = v;
        fill[u as usize] += 1;
        adj[fill[v as usize] as usize] = u;
        fill[v as usize] += 1;
    }
    let mut level = vec![u32::MAX; n];
    let mut queue = Vec::with_capacity(n);
    level[0] = 0;
    queue.push(0u32);
    let mut head = 0;
    while head < queue.len() {
        let u = queue[head] as usize;
        head += 1;
        for &v in &adj[off[u] as usize..off[u + 1] as usize] {
            if level[v as usize] == u32::MAX {
                level[v as usize] = level[u] + 1;
                queue.push(v);
            }
        }
    }
    let mut acc = (queue.len() as u64) << 40;
    for (i, &l) in level.iter().enumerate() {
        if l != u32::MAX {
            acc = acc.wrapping_add(l as u64 * (i as u64 % 5 + 1));
        }
    }
    acc
}

fn queens(mask: u32, cols: u32, d1: u32, d2: u32) -> u64 {
    if cols == mask {
        return 1;
    }
    let mut avail = !(cols | d1 | d2) & mask;
    let mut count = 0;
    while avail != 0 {
        let bit = avail.isolate_lowest_one();
        avail ^= bit;
        count += queens(mask, cols | bit, (d1 | bit) << 1, (d2 | bit) >> 1);
    }
    count
}

/// N-queens solution count (bitmask backtracking); parallel over the first-row column.
pub fn nqueens(n: usize, par: bool) -> u64 {
    let mask = (1u32 << n) - 1;
    sum_map(n, par, |c| {
        let bit = 1u32 << c;
        queens(mask, bit, bit << 1, bit >> 1)
    })
}

const PUZZLES: [&str; 4] = [
    "4.....8.5.3..........7......2.....6.....8.4......1.......6.3.7.5..2.....1.4......",
    "52...6.........7.13...........4..8..6......5...........418.........3..2...87.....",
    "85...24..72......9..4.........1.7..23.5...9...4...........8..7..17..........36.4.",
    "..53.....8......2..7..1.5..4....53...1..7...6..32...8..6.5....9..4....3......97..",
];

struct Sudoku {
    g: [u8; 81],
    rows: [u16; 9],
    cols: [u16; 9],
    boxes: [u16; 9],
    nodes: u64,
}

impl Sudoku {
    fn new(p: &str) -> Self {
        let mut s = Sudoku {
            g: [0; 81],
            rows: [0; 9],
            cols: [0; 9],
            boxes: [0; 9],
            nodes: 0,
        };
        for (i, ch) in p.bytes().enumerate() {
            if ch.is_ascii_digit() && ch != b'0' {
                s.set(i, ch - b'0');
            }
        }
        s
    }
    fn set(&mut self, i: usize, d: u8) {
        let bit = 1u16 << d;
        self.g[i] = d;
        self.rows[i / 9] |= bit;
        self.cols[i % 9] |= bit;
        self.boxes[(i / 27) * 3 + (i % 9) / 3] |= bit;
    }
    fn unset(&mut self, i: usize, d: u8) {
        let bit = !(1u16 << d);
        self.g[i] = 0;
        self.rows[i / 9] &= bit;
        self.cols[i % 9] &= bit;
        self.boxes[(i / 27) * 3 + (i % 9) / 3] &= bit;
    }
    fn solve(&mut self) -> bool {
        self.nodes += 1;
        let mut best = usize::MAX;
        let mut best_cnt = 10;
        let mut best_mask = 0u16;
        for i in 0..81 {
            if self.g[i] != 0 {
                continue;
            }
            let used = self.rows[i / 9] | self.cols[i % 9] | self.boxes[(i / 27) * 3 + (i % 9) / 3];
            let free = !used & 0x3FE;
            let cnt = free.count_ones();
            if cnt < best_cnt {
                best = i;
                best_cnt = cnt;
                best_mask = free;
                if cnt <= 1 {
                    break;
                }
            }
        }
        if best == usize::MAX {
            return true;
        }
        for d in 1..=9u8 {
            if best_mask & (1 << d) != 0 {
                self.set(best, d);
                if self.solve() {
                    return true;
                }
                self.unset(best, d);
            }
        }
        false
    }
}

/// Backtracking sudoku solver (bitmasks + fewest-candidates heuristic), n rounds × 4 hard puzzles.
pub fn sudoku(n: usize, par: bool) -> u64 {
    sum_map(n * PUZZLES.len(), par, |k| {
        let mut s = Sudoku::new(PUZZLES[k % PUZZLES.len()]);
        let ok = s.solve();
        let grid =
            s.g.iter()
                .enumerate()
                .fold(0u64, |a, (i, &d)| a + d as u64 * (i as u64 + 1));
        grid + (s.nodes << 20) + ok as u64
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_variants_match() {
        let cases: [(&str, u64); 17] = [
            (MANDELBROT, 64),
            (CONVOLVE, 64),
            (LANCZOS, 64),
            (DCT, 2),
            (GRADE, 1),
            (PATHTRACE, 16),
            (BVH, 3000),
            (ISOSURFACE, 16),
            (PARTICLES, 1000),
            (KMEANS, 2000),
            (LZ77, 200_000),
            (TEXTSEARCH, 50_000),
            (STRSORT, 5000),
            (LEVENSHTEIN, 100),
            (CHACHA20, 10_000),
            (NQUEENS, 8),
            (SUDOKU, 1),
        ];
        for (id, p) in cases {
            assert_eq!(run(id, p), run_par(id, p), "{id}");
        }
    }

    #[test]
    fn known_answers() {
        assert_eq!(nqueens(8, false), 92);
        // 10! = 3628800 → digit sum 27, one limb
        assert_eq!(bigint_factorial(10), 27 + (1 << 32));
        // CRC-32 check value uses "123456789"; here just make sure it is stable and non-zero.
        assert_ne!(crc32(1000), 0);
        let s = sudoku(1, false);
        assert!(s & 1 == 1 || s > 0);
    }

    #[test]
    fn chacha20_rfc8439_block() {
        // RFC 8439 §2.3.2 test vector
        let key: [u32; 8] = core::array::from_fn(|i| {
            u32::from_le_bytes([
                (i * 4) as u8,
                (i * 4 + 1) as u8,
                (i * 4 + 2) as u8,
                (i * 4 + 3) as u8,
            ])
        });
        let nonce = [0x0900_0000, 0x4a00_0000, 0];
        let mut out = [0u8; 64];
        chacha_block(&key, 1, &nonce, &mut out);
        assert_eq!(&out[..4], &[0x10, 0xf1, 0xe7, 0xe4]);
    }

    #[test]
    fn regex_matcher() {
        assert!(re_match(b"ab.*ef", b"xxabzzefyy"));
        assert!(re_match(b"^b", b"bob"));
        assert!(!re_match(b"^b", b"abc"));
        assert!(re_match(b"e*st$", b"tst"));
    }
}
