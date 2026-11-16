//! Heavy media workloads (Rust side): image rendering, image processing,
//! video processing and 3D model generation. Pure CPU; no GPU, no SIMD intrinsics,
//! no system libraries. Swift counterparts live in
//! `platforms/ios/MasterFabricRust/Benchmarks/SwiftMedia.swift` (same algorithm, same checksum).
//!
//! Each workload is written as row/band kernels. The single-thread variant runs the
//! kernels in a loop; the `_par` variant runs the *same* kernels with rayon, so both
//! produce bit-identical output (and checksum).

use crate::workloads::XorShift32;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fmt::Write;

#[inline]
fn hash_bytes(acc: u64, b: u64) -> u64 {
    acc.wrapping_mul(31).wrapping_add(b)
}

// =================================================================== ray tracer

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
    fn mul(self, s: f64) -> V3 {
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

struct Sphere {
    c: V3,
    r: f64,
    color: V3,
    reflect: f64,
}

const SPHERES: [Sphere; 4] = [
    Sphere {
        c: V3::new(0.0, 0.0, -3.0),
        r: 0.5,
        color: V3::new(0.9, 0.2, 0.2),
        reflect: 0.3,
    },
    Sphere {
        c: V3::new(1.1, 0.0, -3.5),
        r: 0.5,
        color: V3::new(0.2, 0.9, 0.3),
        reflect: 0.3,
    },
    Sphere {
        c: V3::new(-1.1, 0.0, -3.5),
        r: 0.5,
        color: V3::new(0.2, 0.4, 0.9),
        reflect: 0.3,
    },
    Sphere {
        c: V3::new(0.0, 0.9, -4.0),
        r: 0.4,
        color: V3::new(0.95, 0.85, 0.2),
        reflect: 0.5,
    },
];
const PLANE_Y: f64 = -0.5;
const EPS: f64 = 1e-4;

fn hit_sphere(s: &Sphere, o: V3, d: V3) -> Option<f64> {
    let oc = o.sub(s.c);
    let b = oc.dot(d);
    let c = oc.dot(oc) - s.r * s.r;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let sq = disc.sqrt();
    let t0 = -b - sq;
    if t0 > EPS {
        return Some(t0);
    }
    let t1 = -b + sq;
    if t1 > EPS {
        Some(t1)
    } else {
        None
    }
}

fn trace(o: V3, d: V3, depth: u32, light: V3) -> V3 {
    let mut best_t = f64::MAX;
    let mut best: Option<usize> = None;
    for (i, s) in SPHERES.iter().enumerate() {
        if let Some(t) = hit_sphere(s, o, d) {
            if t < best_t {
                best_t = t;
                best = Some(i);
            }
        }
    }
    let mut plane = false;
    if d.y < -1e-9 {
        let t = (PLANE_Y - o.y) / d.y;
        if t > EPS && t < best_t {
            best_t = t;
            plane = true;
            best = None;
        }
    }
    if best.is_none() && !plane {
        let k = 0.5 * (d.y + 1.0);
        return V3::new(1.0, 1.0, 1.0)
            .mul(1.0 - k)
            .add(V3::new(0.5, 0.7, 1.0).mul(k));
    }
    let p = o.add(d.mul(best_t));
    let (n, color, reflect) = if let Some(i) = best {
        let s = &SPHERES[i];
        (p.sub(s.c).mul(1.0 / s.r), s.color, s.reflect)
    } else {
        let parity = (p.x.floor() as i64 + p.z.floor() as i64) & 1;
        let g = if parity == 0 { 0.9 } else { 0.3 };
        (V3::new(0.0, 1.0, 0.0), V3::new(g, g, g), 0.2)
    };
    let mut diffuse = n.dot(light).max(0.0);
    let sp = p.add(n.mul(EPS));
    if SPHERES.iter().any(|s| hit_sphere(s, sp, light).is_some()) {
        diffuse *= 0.3;
    }
    let mut col = color.mul(0.1 + 0.9 * diffuse);
    if depth < 2 && reflect > 0.0 {
        let rd = d.sub(n.mul(2.0 * d.dot(n)));
        col = col
            .mul(1.0 - reflect)
            .add(trace(sp, rd, depth + 1, light).mul(reflect));
    }
    col
}

struct Camera {
    light: V3,
    origin: V3,
}

impl Camera {
    fn new() -> Self {
        Self {
            light: V3::new(-0.6, 1.0, 0.5).norm(),
            origin: V3::new(0.0, 0.2, 0.5),
        }
    }

    /// Shades one row (RGBA8) into `row`.
    fn shade_row(&self, y: usize, w: usize, h: usize, row: &mut [u8]) {
        const OFFSETS: [(f64, f64); 4] = [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)];
        for (x, px) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let mut acc = V3::new(0.0, 0.0, 0.0);
            for (ox, oy) in OFFSETS {
                let u = ((x as f64 + ox) / w as f64) * 2.0 - 1.0;
                let v = 1.0 - ((y as f64 + oy) / h as f64) * 2.0;
                let d = V3::new(u, v, -1.5).norm();
                acc = acc.add(trace(self.origin, d, 0, self.light));
            }
            let c = acc.mul(0.25);
            px[0] = (c.x.clamp(0.0, 1.0).sqrt() * 255.0) as u8;
            px[1] = (c.y.clamp(0.0, 1.0).sqrt() * 255.0) as u8;
            px[2] = (c.z.clamp(0.0, 1.0).sqrt() * 255.0) as u8;
            px[3] = 255;
        }
    }
}

/// RGBA8 image (4 samples/pixel, 2 reflections, shadows). Also shown in the UI as a preview.
pub fn raytrace(w: usize, h: usize) -> Vec<u8> {
    let cam = Camera::new();
    let mut out = vec![0u8; w * h * 4];
    for (y, row) in out.chunks_exact_mut(w * 4).enumerate() {
        cam.shade_row(y, w, h, row);
    }
    out
}

/// Same image, rows rendered in parallel with rayon.
pub fn raytrace_par(w: usize, h: usize) -> Vec<u8> {
    let cam = Camera::new();
    let mut out = vec![0u8; w * h * 4];
    out.par_chunks_exact_mut(w * 4)
        .enumerate()
        .for_each(|(y, row)| cam.shade_row(y, w, h, row));
    out
}

fn hash_all(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0u64, |a, &b| hash_bytes(a, b as u64))
}

pub fn raytrace_par_bench(size: usize) -> u64 {
    hash_all(&raytrace_par(size, size))
}

pub fn raytrace_bench(size: usize) -> u64 {
    raytrace(size, size)
        .iter()
        .fold(0u64, |a, &b| hash_bytes(a, b as u64))
}

// =================================================================== rasterizer

#[inline]
fn edge(ax: i32, ay: i32, bx: i32, by: i32, px: i32, py: i32) -> i32 {
    (bx - ax) * (py - ay) - (by - ay) * (px - ax)
}

const RW: i32 = 512;
const RH: i32 = 512;
const BAND: usize = 16;

#[derive(Clone, Copy)]
struct Tri {
    x: [i32; 3],
    y: [i32; 3],
    z: i32,
    c: u32,
    minx: i32,
    maxx: i32,
    miny: i32,
    maxy: i32,
}

/// Generates the triangle list (same RNG order as the Swift version).
fn make_triangles(n: usize) -> Vec<Tri> {
    let mut rng = XorShift32(0x0BAD_C0DE);
    let mut tris = Vec::with_capacity(n);
    for _ in 0..n {
        let cx = (rng.next_u32() % RW as u32) as i32;
        let cy = (rng.next_u32() % RH as u32) as i32;
        let x0 = cx + (rng.next_u32() % 64) as i32 - 32;
        let y0 = cy + (rng.next_u32() % 64) as i32 - 32;
        let mut x1 = cx + (rng.next_u32() % 64) as i32 - 32;
        let mut y1 = cy + (rng.next_u32() % 64) as i32 - 32;
        let mut x2 = cx + (rng.next_u32() % 64) as i32 - 32;
        let mut y2 = cy + (rng.next_u32() % 64) as i32 - 32;
        let z = (rng.next_u32() % 10_000) as i32;
        let c = rng.next_u32();
        let area = edge(x0, y0, x1, y1, x2, y2);
        if area == 0 {
            continue;
        }
        if area < 0 {
            std::mem::swap(&mut x1, &mut x2);
            std::mem::swap(&mut y1, &mut y2);
        }
        tris.push(Tri {
            x: [x0, x1, x2],
            y: [y0, y1, y2],
            z,
            c,
            minx: x0.min(x1).min(x2).max(0),
            maxx: x0.max(x1).max(x2).min(RW - 1),
            miny: y0.min(y1).min(y2).max(0),
            maxy: y0.max(y1).max(y2).min(RH - 1),
        });
    }
    tris
}

/// Rasterizes all triangles into the rows `[y_start, y_start + rows)`.
/// Triangle order per pixel is preserved, so banding gives identical output.
fn raster_band(tris: &[Tri], y_start: i32, color: &mut [u32], depth: &mut [i32]) {
    let rows = (color.len() / RW as usize) as i32;
    let y_end = y_start + rows - 1;
    for t in tris {
        let (lo, hi) = (t.miny.max(y_start), t.maxy.min(y_end));
        let [x0, x1, x2] = t.x;
        let [y0, y1, y2] = t.y;
        for py in lo..=hi {
            let base = ((py - y_start) * RW) as usize;
            for px in t.minx..=t.maxx {
                let w0 = edge(x1, y1, x2, y2, px, py);
                let w1 = edge(x2, y2, x0, y0, px, py);
                let w2 = edge(x0, y0, x1, y1, px, py);
                if (w0 | w1 | w2) >= 0 {
                    let i = base + px as usize;
                    if t.z < depth[i] {
                        depth[i] = t.z;
                        color[i] = t.c;
                    }
                }
            }
        }
    }
}

/// 512×512 z-buffered triangle rasterizer (edge functions, integer math).
pub fn rasterize(n: usize) -> u64 {
    let tris = make_triangles(n);
    let mut color = vec![0u32; (RW * RH) as usize];
    let mut depth = vec![i32::MAX; (RW * RH) as usize];
    raster_band(&tris, 0, &mut color, &mut depth);
    color.iter().fold(0u64, |a, &c| hash_bytes(a, c as u64))
}

/// Same, with 16-row bands rendered in parallel.
pub fn rasterize_par(n: usize) -> u64 {
    let tris = make_triangles(n);
    let mut color = vec![0u32; (RW * RH) as usize];
    let mut depth = vec![i32::MAX; (RW * RH) as usize];
    let band = BAND * RW as usize;
    color
        .par_chunks_mut(band)
        .zip(depth.par_chunks_mut(band))
        .enumerate()
        .for_each(|(b, (c, d))| raster_band(&tris, (b * BAND) as i32, c, d));
    color.iter().fold(0u64, |a, &c| hash_bytes(a, c as u64))
}

// =================================================================== image processing

const K5: [u32; 5] = [1, 4, 6, 4, 1];

fn image_source(w: usize, h: usize) -> Vec<u8> {
    let mut src = vec![0u8; w * h * 3];
    for (y, row) in src.chunks_exact_mut(w * 3).enumerate() {
        for (x, px) in row.as_chunks_mut::<3>().0.iter_mut().enumerate() {
            px[0] = ((x ^ y) & 0xff) as u8;
            px[1] = ((x * 2 + y) & 0xff) as u8;
            px[2] = (((x * y) >> 4) & 0xff) as u8;
        }
    }
    src
}

/// Horizontal 5-tap blur of one RGB row (edges clamped).
fn blur_h_row(src: &[u8], out: &mut [u8], w: usize) {
    for x in 0..w {
        for ch in 0..3 {
            let mut s = 0u32;
            for (k, kv) in K5.iter().enumerate() {
                let xx = (x + k).saturating_sub(2).min(w - 1);
                s += kv * src[xx * 3 + ch] as u32;
            }
            out[x * 3 + ch] = ((s + 8) >> 4) as u8;
        }
    }
}

/// Vertical 5-tap blur producing row `y`: weighted sum of 5 whole rows (vectorizes well).
fn blur_v_row(tmp: &[u8], out: &mut [u8], y: usize, w: usize, h: usize) {
    let stride = w * 3;
    let r = |k: usize| {
        let yy = (y + k).saturating_sub(2).min(h - 1);
        &tmp[yy * stride..(yy + 1) * stride]
    };
    let (r0, r1, r2, r3, r4) = (r(0), r(1), r(2), r(3), r(4));
    for i in 0..stride {
        let s =
            r0[i] as u32 + 4 * r1[i] as u32 + 6 * r2[i] as u32 + 4 * r3[i] as u32 + r4[i] as u32;
        out[i] = ((s + 8) >> 4) as u8;
    }
}

/// Bilinear (16.16 fixed point) downscale, producing destination row `y`.
fn resize_row(blur: &[u8], out: &mut [u8], y: usize, w: usize, h: usize, dw: usize, dh: usize) {
    let sy = (y as u64 * (((h - 1) as u64) << 16)) / (dh - 1) as u64;
    let y0 = (sy >> 16) as usize;
    let fy = sy & 0xffff;
    let y1 = (y0 + 1).min(h - 1);
    for x in 0..dw {
        let sx = (x as u64 * (((w - 1) as u64) << 16)) / (dw - 1) as u64;
        let x0 = (sx >> 16) as usize;
        let fx = sx & 0xffff;
        let x1 = (x0 + 1).min(w - 1);
        for ch in 0..3 {
            let p00 = blur[(y0 * w + x0) * 3 + ch] as u64;
            let p10 = blur[(y0 * w + x1) * 3 + ch] as u64;
            let p01 = blur[(y1 * w + x0) * 3 + ch] as u64;
            let p11 = blur[(y1 * w + x1) * 3 + ch] as u64;
            let top = p00 * (65536 - fx) + p10 * fx;
            let bot = p01 * (65536 - fx) + p11 * fx;
            out[x * 3 + ch] = ((top * (65536 - fy) + bot * fy) >> 32) as u8;
        }
    }
}

fn grade_lut() -> [u8; 256] {
    let mut lut = [0u8; 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        *slot = (((i as i32 - 128) * 6 / 5) + 138).clamp(0, 255) as u8;
    }
    lut
}

/// Color grade (LUT) + RGB → YUV (BT.601, integer) and checksum of a pixel span.
fn yuv_sum(px: &[u8], lut: &[u8; 256]) -> u64 {
    let mut sum = 0u64;
    for p in px.as_chunks::<3>().0 {
        let r = lut[p[0] as usize] as i32;
        let g = lut[p[1] as usize] as i32;
        let b = lut[p[2] as usize] as i32;
        let yv = ((66 * r + 129 * g + 25 * b + 128) >> 8) + 16;
        let u = ((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128;
        let v = ((112 * r - 94 * g - 18 * b + 128) >> 8) + 128;
        sum += yv as u64 + 3 * u as u64 + 7 * v as u64;
    }
    sum
}

/// 1024×1024 RGB: 5-tap gaussian blur (h+v) → bilinear 512×512 → color LUT → RGB→YUV.
pub fn image_pipeline(size: usize) -> u64 {
    let (w, h) = (size, size);
    let (dw, dh) = (w / 2, h / 2);
    let src = image_source(w, h);
    let mut tmp = vec![0u8; w * h * 3];
    for (y, row) in tmp.chunks_exact_mut(w * 3).enumerate() {
        blur_h_row(&src[y * w * 3..(y + 1) * w * 3], row, w);
    }
    let mut blur = vec![0u8; w * h * 3];
    for (y, row) in blur.chunks_exact_mut(w * 3).enumerate() {
        blur_v_row(&tmp, row, y, w, h);
    }
    let mut small = vec![0u8; dw * dh * 3];
    for (y, row) in small.chunks_exact_mut(dw * 3).enumerate() {
        resize_row(&blur, row, y, w, h, dw, dh);
    }
    yuv_sum(&small, &grade_lut())
}

/// Same pipeline, every stage parallel over rows.
pub fn image_pipeline_par(size: usize) -> u64 {
    let (w, h) = (size, size);
    let (dw, dh) = (w / 2, h / 2);
    let src = image_source(w, h);
    let mut tmp = vec![0u8; w * h * 3];
    tmp.par_chunks_exact_mut(w * 3)
        .enumerate()
        .for_each(|(y, row)| blur_h_row(&src[y * w * 3..(y + 1) * w * 3], row, w));
    let mut blur = vec![0u8; w * h * 3];
    blur.par_chunks_exact_mut(w * 3)
        .enumerate()
        .for_each(|(y, row)| blur_v_row(&tmp, row, y, w, h));
    let mut small = vec![0u8; dw * dh * 3];
    small
        .par_chunks_exact_mut(dw * 3)
        .enumerate()
        .for_each(|(y, row)| resize_row(&blur, row, y, w, h, dw, dh));
    let lut = grade_lut();
    small.par_chunks(dw * 3).map(|row| yuv_sum(row, &lut)).sum()
}

// =================================================================== video processing

const VW: usize = 640;
const VH: usize = 360;
const VB: usize = 16;

/// Synthetic RGB frame `f`, converted to luma, row `y`.
fn video_luma_row(f: usize, y: usize, row: &mut [u8]) {
    let (sqx, sqy) = (40 + f * 6, 60 + f * 3);
    for (x, out) in row.iter_mut().enumerate() {
        let (r, g, b) = if x >= sqx && x < sqx + 80 && y >= sqy && y < sqy + 80 {
            (240u32, 240u32, 240u32)
        } else {
            (
                ((x + f * 4) & 0xff) as u32,
                ((y * 2 + f * 2) & 0xff) as u32,
                (((x + y) / 2) & 0xff) as u32,
            )
        };
        *out = (((66 * r + 129 * g + 25 * b + 128) >> 8) + 16) as u8;
    }
}

/// 3×3 box blur for row `y` (border pixels copied).
fn video_blur_row(luma: &[u8], y: usize, out: &mut [u8]) {
    let row = |yy: usize| &luma[yy * VW..(yy + 1) * VW];
    if y == 0 || y == VH - 1 {
        out.copy_from_slice(row(y));
        return;
    }
    let (a, b, c) = (row(y - 1), row(y), row(y + 1));
    out[0] = b[0];
    out[VW - 1] = b[VW - 1];
    for x in 1..VW - 1 {
        let s = a[x - 1] as u32
            + a[x] as u32
            + a[x + 1] as u32
            + b[x - 1] as u32
            + b[x] as u32
            + b[x + 1] as u32
            + c[x - 1] as u32
            + c[x] as u32
            + c[x + 1] as u32;
        out[x] = (s / 9) as u8;
    }
}

#[inline]
fn sad16(cur: &[u8], prev: &[u8]) -> u64 {
    cur.iter()
        .zip(prev)
        .map(|(&a, &b)| a.abs_diff(b) as u32)
        .sum::<u32>() as u64
}

/// Block motion estimation for one row of 16×16 blocks (±2 px search).
/// Returns (sum of best SADs, sum of dx, sum of dy).
fn motion_row(cur: &[u8], prev: &[u8], by: usize) -> (u64, i64, i64) {
    let (mut sad_total, mut mvx, mut mvy) = (0u64, 0i64, 0i64);
    for bx in 0..VW / VB {
        let (ox, oy) = ((bx * VB) as i64, (by * VB) as i64);
        let mut best = u64::MAX;
        let (mut bdx, mut bdy) = (0i64, 0i64);
        for dy in -2i64..=2 {
            for dx in -2i64..=2 {
                let (px, py) = (ox + dx, oy + dy);
                if px < 0 || py < 0 || px + VB as i64 > VW as i64 || py + VB as i64 > VH as i64 {
                    continue;
                }
                let mut sad = 0u64;
                for yy in 0..VB {
                    let c = (oy as usize + yy) * VW + ox as usize;
                    let p = (py as usize + yy) * VW + px as usize;
                    sad += sad16(&cur[c..c + VB], &prev[p..p + VB]);
                }
                if sad < best {
                    best = sad;
                    bdx = dx;
                    bdy = dy;
                }
            }
        }
        sad_total += best;
        mvx += bdx;
        mvy += bdy;
    }
    (sad_total, mvx, mvy)
}

fn frame_diff(cur: &[u8], prev: &[u8]) -> u64 {
    cur.iter()
        .zip(prev)
        .map(|(&a, &b)| a.abs_diff(b) as u64)
        .sum()
}

fn video_checksum(diff: u64, sad: u64, mvx: i64, mvy: i64) -> u64 {
    hash_bytes(
        hash_bytes(hash_bytes(diff, sad), (mvx + 100_000) as u64),
        (mvy + 100_000) as u64,
    )
}

/// `frames` synthetic 640×360 frames: RGB→Y, 3×3 blur, frame difference and
/// 16×16 block motion estimation (±2 px search).
pub fn video_pipeline(frames: usize) -> u64 {
    let mut luma = vec![0u8; VW * VH];
    let mut cur = vec![0u8; VW * VH];
    let mut prev = vec![0u8; VW * VH];
    let (mut diff, mut sad, mut mvx, mut mvy) = (0u64, 0u64, 0i64, 0i64);
    for f in 0..frames {
        for (y, row) in luma.as_chunks_mut::<VW>().0.iter_mut().enumerate() {
            video_luma_row(f, y, row);
        }
        for (y, row) in cur.as_chunks_mut::<VW>().0.iter_mut().enumerate() {
            video_blur_row(&luma, y, row);
        }
        if f > 0 {
            diff += frame_diff(&cur, &prev);
            for by in 0..VH / VB {
                let (s, x, y) = motion_row(&cur, &prev, by);
                sad += s;
                mvx += x;
                mvy += y;
            }
        }
        std::mem::swap(&mut cur, &mut prev);
    }
    video_checksum(diff, sad, mvx, mvy)
}

/// Same pipeline; each stage within a frame runs in parallel (rows / block rows).
pub fn video_pipeline_par(frames: usize) -> u64 {
    let mut luma = vec![0u8; VW * VH];
    let mut cur = vec![0u8; VW * VH];
    let mut prev = vec![0u8; VW * VH];
    let (mut diff, mut sad, mut mvx, mut mvy) = (0u64, 0u64, 0i64, 0i64);
    for f in 0..frames {
        luma.par_chunks_exact_mut(VW)
            .enumerate()
            .for_each(|(y, row)| video_luma_row(f, y, row));
        cur.par_chunks_exact_mut(VW)
            .enumerate()
            .for_each(|(y, row)| video_blur_row(&luma, y, row));
        if f > 0 {
            diff += cur
                .par_chunks(VW)
                .zip(prev.par_chunks(VW))
                .map(|(a, b)| frame_diff(a, b))
                .sum::<u64>();
            let (s, x, y) = (0..VH / VB)
                .into_par_iter()
                .map(|by| motion_row(&cur, &prev, by))
                .reduce(|| (0, 0, 0), |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2));
            sad += s;
            mvx += x;
            mvy += y;
        }
        std::mem::swap(&mut cur, &mut prev);
    }
    video_checksum(diff, sad, mvx, mvy)
}

// =================================================================== 3D model generation

/// Subdivided icosphere + sine-noise surface displacement + vertex normals
/// + in-memory OBJ text. Returns (vertices, triangles, OBJ bytes, checksum).
pub fn generate_mesh(levels: u32) -> (usize, usize, usize, u64) {
    let t = (1.0 + 5.0f64.sqrt()) / 2.0;
    let mut verts: Vec<[f64; 3]> = vec![
        [-1.0, t, 0.0],
        [1.0, t, 0.0],
        [-1.0, -t, 0.0],
        [1.0, -t, 0.0],
        [0.0, -1.0, t],
        [0.0, 1.0, t],
        [0.0, -1.0, -t],
        [0.0, 1.0, -t],
        [t, 0.0, -1.0],
        [t, 0.0, 1.0],
        [-t, 0.0, -1.0],
        [-t, 0.0, 1.0],
    ];
    for v in verts.iter_mut() {
        *v = normalize(*v);
    }
    let mut faces: Vec<[u32; 3]> = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    for _ in 0..levels {
        let mut cache: HashMap<u64, u32> = HashMap::new();
        let mut next = Vec::with_capacity(faces.len() * 4);
        for f in &faces {
            let a = midpoint(&mut verts, &mut cache, f[0], f[1]);
            let b = midpoint(&mut verts, &mut cache, f[1], f[2]);
            let c = midpoint(&mut verts, &mut cache, f[2], f[0]);
            next.push([f[0], a, c]);
            next.push([f[1], b, a]);
            next.push([f[2], c, b]);
            next.push([a, b, c]);
        }
        faces = next;
    }
    for v in verts.iter_mut() {
        let [x, y, z] = *v;
        let d = 1.0
            + 0.08 * (5.0 * x).sin() * (5.0 * y).sin() * (5.0 * z).sin()
            + 0.03 * (13.0 * x + 7.0 * y).sin();
        *v = [x * d, y * d, z * d];
    }
    let mut normals = vec![[0.0f64; 3]; verts.len()];
    for f in &faces {
        let (a, b, c) = (
            verts[f[0] as usize],
            verts[f[1] as usize],
            verts[f[2] as usize],
        );
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        for &i in f {
            let slot = &mut normals[i as usize];
            slot[0] += n[0];
            slot[1] += n[1];
            slot[2] += n[2];
        }
    }
    for n in normals.iter_mut() {
        *n = normalize(*n);
    }
    let mut obj = String::with_capacity(verts.len() * 64 + faces.len() * 32);
    for v in &verts {
        let _ = writeln!(obj, "v {:.4} {:.4} {:.4}", v[0], v[1], v[2]);
    }
    for n in &normals {
        let _ = writeln!(obj, "vn {:.4} {:.4} {:.4}", n[0], n[1], n[2]);
    }
    for f in &faces {
        let (a, b, c) = (f[0] + 1, f[1] + 1, f[2] + 1);
        let _ = writeln!(obj, "f {a}//{a} {b}//{b} {c}//{c}");
    }
    let mut q: i64 = 0;
    for (v, n) in verts.iter().zip(&normals) {
        q = q.wrapping_add(
            (v[0] * 1e6).round() as i64 + (v[1] * 1e6).round() as i64 + (v[2] * 1e6).round() as i64,
        );
        q = q.wrapping_add(
            (n[0] * 1e3).round() as i64 + (n[1] * 1e3).round() as i64 + (n[2] * 1e3).round() as i64,
        );
    }
    let checksum = hash_bytes(hash_bytes(verts.len() as u64, faces.len() as u64), q as u64);
    (verts.len(), faces.len(), obj.len(), checksum)
}

fn normalize(v: [f64; 3]) -> [f64; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / l, v[1] / l, v[2] / l]
}

fn midpoint(verts: &mut Vec<[f64; 3]>, cache: &mut HashMap<u64, u32>, a: u32, b: u32) -> u32 {
    let key = ((a.min(b) as u64) << 32) | a.max(b) as u64;
    if let Some(&i) = cache.get(&key) {
        return i;
    }
    let (p, q) = (verts[a as usize], verts[b as usize]);
    verts.push(normalize([
        (p[0] + q[0]) * 0.5,
        (p[1] + q[1]) * 0.5,
        (p[2] + q[2]) * 0.5,
    ]));
    let i = (verts.len() - 1) as u32;
    cache.insert(key, i);
    i
}

pub fn mesh_bench(levels: u32) -> u64 {
    generate_mesh(levels).3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icosphere_counts() {
        let (v, f, obj, _) = generate_mesh(3);
        assert_eq!(v, 642);
        assert_eq!(f, 1280);
        assert!(obj > 0);
    }

    #[test]
    fn raytrace_has_content() {
        let img = raytrace(32, 32);
        assert_eq!(img.len(), 32 * 32 * 4);
        assert!(img
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[0] > 150 && p[1] < 120)); // red sphere visible
    }

    #[test]
    fn parallel_matches_single_thread() {
        assert_eq!(raytrace(48, 48), raytrace_par(48, 48));
        assert_eq!(rasterize(500), rasterize_par(500));
        assert_eq!(image_pipeline(128), image_pipeline_par(128));
        assert_eq!(video_pipeline(4), video_pipeline_par(4));
    }

    #[test]
    fn deterministic() {
        assert_eq!(rasterize(200), rasterize(200));
        assert_eq!(video_pipeline(3), video_pipeline(3));
        assert_eq!(image_pipeline(64), image_pipeline(64));
    }
}
