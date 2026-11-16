import Foundation

// One-to-one Swift port of `crates/app-core/src/heavy.rs` (single-thread versions).
// Same algorithms, same data, same checksums. Idiomatic Swift: arrays, structs, closures,
// standard library collections; no unsafe pointers, no Accelerate.

enum SwiftHeavy {
    static func run(_ id: String, _ p: UInt64) -> UInt64? {
        let n = Int(p)
        switch id {
        case "mandelbrot": return mandelbrot(n)
        case "convolve": return convolve(n)
        case "lanczos": return lanczos(n)
        case "dct": return dct(n)
        case "grade": return grade(n)
        case "pathtrace": return pathtrace(n)
        case "bvh": return bvh(n)
        case "isosurface": return isosurface(n)
        case "nbody": return nbody(n)
        case "fft": return fft(n)
        case "bigint": return bigintFactorial(n)
        case "particles": return particles(n)
        case "kmeans": return kmeans(n)
        case "lz77": return lz77(n)
        case "wordfreq": return wordfreq(n)
        case "textsearch": return textsearch(n)
        case "strsort": return strsort(n)
        case "levenshtein": return levenshtein(n)
        case "chacha20": return chacha20(n)
        case "crc32": return crc32(n)
        case "dijkstra": return dijkstra(n)
        case "bfs": return bfs(n)
        case "nqueens": return nqueens(n)
        case "sudoku": return sudoku(n)
        default: return nil
        }
    }

    // MARK: helpers

    @inline(__always) static func q(_ x: Double, _ s: Double) -> UInt64 {
        UInt64(bitPattern: Int64((x * s).rounded()))
    }

    @inline(__always) static func rnd01(_ r: inout XorShift32) -> Double {
        Double(r.next() >> 8) / 16_777_216.0
    }

    static func fnv(_ bytes: [UInt8]) -> UInt64 {
        var h: UInt64 = 0xcbf2_9ce4_8422_2325
        for b in bytes {
            h ^= UInt64(b)
            h = h &* 0x0000_0100_0000_01b3
        }
        return h
    }

    static func weightedSum(_ bytes: [UInt8], _ m: UInt64) -> UInt64 {
        var a: UInt64 = 0
        for (i, v) in bytes.enumerated() { a = a &+ UInt64(v) &* (UInt64(i) % m + 1) }
        return a
    }

    // MARK: Image

    static func mandelbrot(_ n: Int) -> UInt64 {
        var img = [UInt16](repeating: 0, count: n * n)
        for y in 0..<n {
            let cy = -1.2 + 2.4 * Double(y) / Double(n)
            for x in 0..<n {
                let cx = -2.2 + 3.2 * Double(x) / Double(n)
                var zx = 0.0, zy = 0.0
                var i: UInt16 = 0
                while i < 256 {
                    let zx2 = zx * zx
                    let zy2 = zy * zy
                    if zx2 + zy2 > 4.0 { break }
                    zy = 2.0 * zx * zy + cy
                    zx = zx2 - zy2 + cx
                    i += 1
                }
                img[y * n + x] = i
            }
        }
        var a: UInt64 = 0
        for (i, v) in img.enumerated() { a = a &+ UInt64(v) &* (UInt64(i) % 7 + 1) }
        return a
    }

    static func graySource(_ n: Int, _ seed: UInt32) -> [UInt8] {
        var r = XorShift32(state: seed)
        var v = [UInt8]()
        v.reserveCapacity(n * n)
        for i in 0..<(n * n) {
            let x = UInt32(i % n)
            let y = UInt32(i / n)
            v.append(UInt8(((x ^ y) &+ (r.next() & 63)) & 255))
        }
        return v
    }

    static let k5: [UInt32] = [1, 4, 6, 4, 1]

    static func convolve(_ n: Int) -> UInt64 {
        let src = graySource(n, 0x9E37_79B9)
        var blur = [UInt8](repeating: 0, count: n * n)
        for y in 2..<(n - 2) {
            for x in 2..<(n - 2) {
                var s: UInt32 = 0
                for j in 0..<5 {
                    let base = (y + j - 2) * n + x - 2
                    var t: UInt32 = 0
                    for i in 0..<5 { t += k5[i] * UInt32(src[base + i]) }
                    s += k5[j] * t
                }
                blur[y * n + x] = UInt8(s >> 8)
            }
        }
        var edges = [UInt8](repeating: 0, count: n * n)
        for y in 1..<(n - 1) {
            for x in 1..<(n - 1) {
                func p(_ dx: Int, _ dy: Int) -> Int { Int(blur[(y + dy - 1) * n + x + dx - 1]) }
                let gx = -p(0, 0) - 2 * p(0, 1) - p(0, 2) + p(2, 0) + 2 * p(2, 1) + p(2, 2)
                let gy = -p(0, 0) - 2 * p(1, 0) - p(2, 0) + p(0, 2) + 2 * p(1, 2) + p(2, 2)
                edges[y * n + x] = UInt8(min(abs(gx) + abs(gy), 255))
            }
        }
        return weightedSum(edges, 13)
    }

    static func lanczosK(_ t: Double) -> Double {
        if t == 0 { return 1 }
        if abs(t) < 3 {
            let pt = Double.pi * t
            return 3.0 * sin(pt) * sin(pt / 3.0) / (pt * pt)
        }
        return 0
    }

    static func lanczos(_ n: Int) -> UInt64 {
        let src = graySource(n, 0x1357_9BDF)
        let m = n * 3 / 4
        let scale = Double(n) / Double(m)
        let support = 3.0 * scale
        let taps = Int((support * 2.0).rounded(.up)) + 1
        var starts = [Int]()
        var ws = [Double]()
        for x in 0..<m {
            let center = (Double(x) + 0.5) * scale - 0.5
            let start = Int((center - support).rounded(.up))
            let first = ws.count
            var sum = 0.0
            for k in 0..<taps {
                let w = lanczosK((Double(start + k) - center) / scale)
                ws.append(w)
                sum += w
            }
            for i in first..<ws.count { ws[i] /= sum }
            starts.append(start)
        }
        let last = n - 1
        var tmp = [Double](repeating: 0, count: n * m)
        for y in 0..<n {
            for x in 0..<m {
                var acc = 0.0
                for k in 0..<taps {
                    let j = min(max(starts[x] + k, 0), last)
                    acc += ws[x * taps + k] * Double(src[y * n + j])
                }
                tmp[y * m + x] = acc
            }
        }
        var out = [UInt8](repeating: 0, count: m * m)
        for y in 0..<m {
            for x in 0..<m {
                var acc = 0.0
                for k in 0..<taps {
                    let j = min(max(starts[y] + k, 0), last)
                    acc += ws[y * taps + k] * tmp[j * m + x]
                }
                out[y * m + x] = UInt8(min(max(acc.rounded(), 0), 255))
            }
        }
        return weightedSum(out, 11)
    }

    // MARK: Video

    static let qy: [Double] = [
        16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69,
        56, 14, 17, 22, 29, 51, 87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104,
        113, 92, 49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98, 112, 100, 103, 99,
    ]

    static func dct(_ frames: Int) -> UInt64 {
        var cosT = [Double](repeating: 0, count: 64)
        for x in 0..<8 {
            for u in 0..<8 { cosT[x * 8 + u] = cos(Double(2 * x + 1) * Double(u) * Double.pi / 16.0) }
        }
        let w = 640, h = 360
        var total: UInt64 = 0
        var luma = [UInt8](repeating: 0, count: w * h)
        var tmp = [Double](repeating: 0, count: 64)
        for f in 0..<frames {
            for y in 0..<h {
                for x in 0..<w {
                    let base = UInt32((x + f * 4) ^ (y * 2)) & 255
                    let noise = (UInt32(truncatingIfNeeded: x * 7 + y * 13 + f * 31) &* 2_654_435_761) >> 27
                    luma[y * w + x] = UInt8((base &+ noise) & 255)
                }
            }
            var acc: UInt64 = 0
            var nz: UInt64 = 0
            for by in 0..<(h / 8) {
                for bx in 0..<(w / 8) {
                    for y in 0..<8 {
                        for u in 0..<8 {
                            var s = 0.0
                            for x in 0..<8 {
                                let p = Double(luma[(by * 8 + y) * w + bx * 8 + x]) - 128.0
                                s += p * cosT[x * 8 + u]
                            }
                            tmp[y * 8 + u] = s
                        }
                    }
                    for v in 0..<8 {
                        for u in 0..<8 {
                            var s = 0.0
                            for y in 0..<8 { s += tmp[y * 8 + u] * cosT[y * 8 + v] }
                            let cu = u == 0 ? 0.7071067811865476 : 1.0
                            let cv = v == 0 ? 0.7071067811865476 : 1.0
                            let val = 0.25 * cu * cv * s
                            let qv = Int64((val / qy[v * 8 + u]).rounded())
                            acc = acc &+ UInt64(bitPattern: qv &* Int64(v * 8 + u + 1))
                            if qv != 0 { nz += 1 }
                        }
                    }
                }
            }
            total = total &+ (acc ^ (nz << 40))
        }
        return total
    }

    static func grade(_ frames: Int) -> UInt64 {
        var lut = [Int](repeating: 0, count: 256)
        for i in 0..<256 { lut[i] = Int((pow(Double(i) / 255.0, 0.8) * 255.0).rounded()) }
        let w = 1280, h = 720
        let maxD2 = (w / 2) * (w / 2) + (h / 2) * (h / 2)
        var total: UInt64 = 0
        for f in 0..<frames {
            var hist = [UInt32](repeating: 0, count: 256)
            var acc: UInt64 = 0
            for y in 0..<h {
                for x in 0..<w {
                    var r = lut[(x + f * 3) & 255]
                    var g = lut[(y + f * 2) & 255]
                    var b = lut[((x ^ y) + f) & 255]
                    let l = (77 * r + 150 * g + 29 * b) >> 8
                    r = min(max(l + (r - l) * 5 / 4, 0), 255)
                    g = min(max(l + (g - l) * 5 / 4, 0), 255)
                    b = min(max(l + (b - l) * 5 / 4, 0), 255)
                    let dx = x - w / 2
                    let dy = y - h / 2
                    let factor = 256 - (dx * dx + dy * dy) * 96 / maxD2
                    r = (r * factor) >> 8
                    g = (g * factor) >> 8
                    b = (b * factor) >> 8
                    hist[(77 * r + 150 * g + 29 * b) >> 8] += 1
                    acc = acc &+ UInt64(r ^ (g << 8) ^ (b << 16))
                }
            }
            for (i, c) in hist.enumerated() { acc = acc &+ UInt64(c) &* UInt64(i + 1) }
            total = total &+ acc
        }
        return total
    }

    // MARK: 3D

    struct V3 {
        var x, y, z: Double
        init(_ x: Double, _ y: Double, _ z: Double) { self.x = x; self.y = y; self.z = z }
        func add(_ o: V3) -> V3 { V3(x + o.x, y + o.y, z + o.z) }
        func sub(_ o: V3) -> V3 { V3(x - o.x, y - o.y, z - o.z) }
        func mul(_ o: V3) -> V3 { V3(x * o.x, y * o.y, z * o.z) }
        func scale(_ s: Double) -> V3 { V3(x * s, y * s, z * s) }
        func dot(_ o: V3) -> Double { x * o.x + y * o.y + z * o.z }
        func norm() -> V3 { let l = dot(self).squareRoot(); return V3(x / l, y / l, z / l) }
    }

    static let ptSpheres: [(c: V3, r: Double, albedo: V3)] = [
        (V3(0, -1000.5, -1), 1000, V3(0.5, 0.5, 0.5)),
        (V3(0, 0, -1), 0.5, V3(0.7, 0.3, 0.3)),
        (V3(-1.05, 0, -1), 0.5, V3(0.3, 0.7, 0.3)),
        (V3(1.05, 0, -1), 0.5, V3(0.3, 0.3, 0.8)),
        (V3(0, -0.35, -0.45), 0.15, V3(0.9, 0.9, 0.2)),
    ]

    static func ptTrace(_ o0: V3, _ d0: V3, _ r: inout XorShift32) -> V3 {
        var o = o0, d = d0
        var thr = V3(1, 1, 1)
        for _ in 0..<4 {
            var best = Double.greatestFiniteMagnitude
            var hit = -1
            for (i, s) in ptSpheres.enumerated() {
                let oc = o.sub(s.c)
                let b = oc.dot(d)
                let cc = oc.dot(oc) - s.r * s.r
                let disc = b * b - cc
                if disc < 0 { continue }
                let sq = disc.squareRoot()
                var t = -b - sq
                if t < 1e-4 { t = -b + sq }
                if t >= 1e-4 && t < best { best = t; hit = i }
            }
            if hit < 0 {
                let t = 0.5 * (d.y + 1.0)
                let sky = V3(1.0 - t + t * 0.5, 1.0 - t + t * 0.7, 1.0 - t + t * 1.0)
                return thr.mul(sky)
            }
            let s = ptSpheres[hit]
            let p = o.add(d.scale(best))
            let nrm = p.sub(s.c).scale(1.0 / s.r)
            var ru: V3
            while true {
                let v = V3(rnd01(&r) * 2.0 - 1.0, rnd01(&r) * 2.0 - 1.0, rnd01(&r) * 2.0 - 1.0)
                let l2 = v.dot(v)
                if l2 < 1.0 && l2 > 1e-6 { ru = v.norm(); break }
            }
            thr = thr.mul(s.albedo)
            o = p
            d = nrm.add(ru).norm()
        }
        return V3(0, 0, 0)
    }

    static func pathtrace(_ n: Int) -> UInt64 {
        let spp = 8
        var img = [UInt8](repeating: 0, count: n * n * 3)
        for y in 0..<n {
            for x in 0..<n {
                let seed = UInt32(truncatingIfNeeded: y * n + x) &* 2_654_435_761 ^ 0xA5A5_5A5A
                var r = XorShift32(state: seed == 0 ? 1 : seed)
                var col = V3(0, 0, 0)
                for _ in 0..<spp {
                    let u = ((Double(x) + rnd01(&r)) / Double(n)) * 2.0 - 1.0
                    let v = 1.0 - ((Double(y) + rnd01(&r)) / Double(n)) * 2.0
                    let d = V3(u * 1.2, v * 1.2, -1.0).norm()
                    col = col.add(ptTrace(V3(0, 0, 0), d, &r))
                }
                let base = (y * n + x) * 3
                for (k, c) in [col.x, col.y, col.z].enumerated() {
                    img[base + k] = UInt8(min(max((c / Double(spp)).squareRoot(), 0), 0.999) * 256.0)
                }
            }
        }
        return weightedSum(img, 13)
    }

    final class BVHBuilder {
        var bmin: [Double], bmax: [Double], cent: [Double], idx: [Int]
        var nodes: UInt64 = 0, leaves: UInt64 = 0, depthSum: UInt64 = 0, areaQ: UInt64 = 0
        init(bmin: [Double], bmax: [Double], cent: [Double]) {
            self.bmin = bmin; self.bmax = bmax; self.cent = cent
            idx = Array(0..<(cent.count / 3))
        }

        func build(_ lo: Int, _ hi: Int, _ depth: UInt64) {
            var mn = [Double](repeating: .greatestFiniteMagnitude, count: 3)
            var mx = [Double](repeating: -.greatestFiniteMagnitude, count: 3)
            var cmn = mn, cmx = mx
            for k in lo..<hi {
                let i = idx[k]
                for a in 0..<3 {
                    if bmin[i * 3 + a] < mn[a] { mn[a] = bmin[i * 3 + a] }
                    if bmax[i * 3 + a] > mx[a] { mx[a] = bmax[i * 3 + a] }
                    if cent[i * 3 + a] < cmn[a] { cmn[a] = cent[i * 3 + a] }
                    if cent[i * 3 + a] > cmx[a] { cmx[a] = cent[i * 3 + a] }
                }
            }
            let dx = mx[0] - mn[0], dy = mx[1] - mn[1], dz = mx[2] - mn[2]
            nodes += 1
            areaQ = areaQ &+ SwiftHeavy.q(2.0 * (dx * dy + dy * dz + dz * dx), 10.0)
            if hi - lo <= 4 { leaves += 1; depthSum += depth; return }
            let ext = [cmx[0] - cmn[0], cmx[1] - cmn[1], cmx[2] - cmn[2]]
            var axis = 0
            if ext[1] > ext[axis] { axis = 1 }
            if ext[2] > ext[axis] { axis = 2 }
            let c = cent
            idx[lo..<hi].sort { a, b in
                let ca = c[a * 3 + axis], cb = c[b * 3 + axis]
                return ca < cb || (ca == cb && a < b)
            }
            let mid = lo + (hi - lo) / 2
            build(lo, mid, depth + 1)
            build(mid, hi, depth + 1)
        }
    }

    static func bvh(_ n: Int) -> UInt64 {
        var r = XorShift32(state: 0xB0B0_CAFE)
        var bmin = [Double](), bmax = [Double](), cent = [Double]()
        bmin.reserveCapacity(n * 3); bmax.reserveCapacity(n * 3); cent.reserveCapacity(n * 3)
        for _ in 0..<n {
            let base = [rnd01(&r) * 100.0, rnd01(&r) * 100.0, rnd01(&r) * 100.0]
            var v = [[Double]](repeating: [0, 0, 0], count: 3)
            for k in 0..<3 {
                for a in 0..<3 { v[k][a] = base[a] + (rnd01(&r) - 0.5) * 2.0 }
            }
            for a in 0..<3 {
                var mn = v[0][a], mx = v[0][a]
                for k in 1..<3 {
                    if v[k][a] < mn { mn = v[k][a] }
                    if v[k][a] > mx { mx = v[k][a] }
                }
                bmin.append(mn); bmax.append(mx)
                cent.append((v[0][a] + v[1][a] + v[2][a]) / 3.0)
            }
        }
        let b = BVHBuilder(bmin: bmin, bmax: bmax, cent: cent)
        b.build(0, n, 0)
        return (b.nodes ^ (b.leaves << 20) ^ (b.depthSum << 32)) &+ b.areaQ
    }

    static let balls: [(Double, Double, Double, Double)] = [
        (-0.3, 0.0, 0.0, 0.35), (0.35, 0.1, 0.0, 0.3), (0.0, -0.25, 0.3, 0.25),
    ]
    static let cubeEdges: [(Int, Int)] = [
        (0, 1), (2, 3), (4, 5), (6, 7), (0, 2), (1, 3), (4, 6), (5, 7), (0, 4), (1, 5), (2, 6), (3, 7),
    ]

    static func isosurface(_ n: Int) -> UInt64 {
        let g = n + 1
        var field = [Double](repeating: 0, count: g * g * g)
        for k in 0..<g {
            let pz = Double(k) / Double(n) * 2.0 - 1.0
            for j in 0..<g {
                let py = Double(j) / Double(n) * 2.0 - 1.0
                for i in 0..<g {
                    let px = Double(i) / Double(n) * 2.0 - 1.0
                    var f = 0.0
                    for (bx, by, bz, br) in balls {
                        let dx = px - bx, dy = py - by, dz = pz - bz
                        f += br * br / (dx * dx + dy * dy + dz * dz + 1e-6)
                    }
                    field[(k * g + j) * g + i] = f - 1.0
                }
            }
        }
        func at(_ i: Int, _ j: Int, _ k: Int) -> Double { field[(k * g + j) * g + i] }
        var vsum: UInt64 = 0
        var v = [Double](repeating: 0, count: 8)
        for k in 0..<n {
            var verts: UInt64 = 0
            var acc: UInt64 = 0
            for j in 0..<n {
                for i in 0..<n {
                    var mask = 0
                    for c in 0..<8 {
                        v[c] = at(i + (c & 1), j + ((c >> 1) & 1), k + (c >> 2))
                        if v[c] > 0 { mask |= 1 << c }
                    }
                    if mask == 0 || mask == 255 { continue }
                    var sx = 0.0, sy = 0.0, sz = 0.0, cnt = 0.0
                    for (a, b) in cubeEdges where (v[a] > 0) != (v[b] > 0) {
                        let t = v[a] / (v[a] - v[b])
                        let pa = [Double(a & 1), Double((a >> 1) & 1), Double(a >> 2)]
                        let pb = [Double(b & 1), Double((b >> 1) & 1), Double(b >> 2)]
                        sx += pa[0] + (pb[0] - pa[0]) * t
                        sy += pa[1] + (pb[1] - pa[1]) * t
                        sz += pa[2] + (pb[2] - pa[2]) * t
                        cnt += 1.0
                    }
                    verts += 1
                    acc = acc &+ q(Double(i) + sx / cnt, 1000.0) &+ q(Double(j) + sy / cnt, 1000.0)
                        &+ q(Double(k) + sz / cnt, 1000.0)
                }
            }
            vsum = vsum &+ (acc &+ (verts << 40))
        }
        var quads: UInt64 = 0
        for k in 1..<n {
            for j in 1..<n {
                for i in 0..<n {
                    if (at(i, j, k) > 0) != (at(i + 1, j, k) > 0) { quads += 1 }
                    if (at(j, i, k) > 0) != (at(j, i + 1, k) > 0) { quads += 1 }
                    if (at(j, k, i) > 0) != (at(j, k, i + 1) > 0) { quads += 1 }
                }
            }
        }
        return vsum &+ quads * 2
    }

    // MARK: Math

    static func nbody(_ steps: Int) -> UInt64 {
        let sm = 4.0 * Double.pi * Double.pi
        let dpy = 365.24
        var b: [[Double]] = [
            [0, 0, 0, 0, 0, 0, sm],
            [4.84143144246472e0, -1.1603200440274284e0, -1.0362204447112311e-1,
             1.660076642744037e-3 * dpy, 7.699011184197404e-3 * dpy, -6.90460016972063e-5 * dpy,
             9.547919384243266e-4 * sm],
            [8.34336671824458e0, 4.124798564124305e0, -4.035234171143214e-1,
             -2.767425107268624e-3 * dpy, 4.998528012349172e-3 * dpy, 2.3041729757376393e-5 * dpy,
             2.858859806661308e-4 * sm],
            [1.2894369562139131e1, -1.5111151401698631e1, -2.2330757889265573e-1,
             2.964601375647616e-3 * dpy, 2.3784717395948095e-3 * dpy, -2.9658956854023756e-5 * dpy,
             4.366244043351563e-5 * sm],
            [1.5379697114850911e1, -2.5919314609987964e1, 1.7925877295037118e-1,
             2.6806777249038932e-3 * dpy, 1.628241700382423e-3 * dpy, -9.515922545197159e-5 * dpy,
             5.1513890204661145e-5 * sm],
        ]
        var px = 0.0, py = 0.0, pz = 0.0
        for body in b {
            px += body[3] * body[6]
            py += body[4] * body[6]
            pz += body[5] * body[6]
        }
        b[0][3] = -px / sm
        b[0][4] = -py / sm
        b[0][5] = -pz / sm
        let dt = 0.01
        for _ in 0..<steps {
            for i in 0..<5 {
                for j in (i + 1)..<5 {
                    let dx = b[i][0] - b[j][0]
                    let dy = b[i][1] - b[j][1]
                    let dz = b[i][2] - b[j][2]
                    let d2 = dx * dx + dy * dy + dz * dz
                    let mag = dt / (d2 * d2.squareRoot())
                    let mj = b[j][6] * mag
                    let mi = b[i][6] * mag
                    b[i][3] -= dx * mj
                    b[i][4] -= dy * mj
                    b[i][5] -= dz * mj
                    b[j][3] += dx * mi
                    b[j][4] += dy * mi
                    b[j][5] += dz * mi
                }
            }
            for i in 0..<5 {
                b[i][0] += dt * b[i][3]
                b[i][1] += dt * b[i][4]
                b[i][2] += dt * b[i][5]
            }
        }
        var e = 0.0
        for i in 0..<5 {
            e += 0.5 * b[i][6] * (b[i][3] * b[i][3] + b[i][4] * b[i][4] + b[i][5] * b[i][5])
            for j in (i + 1)..<5 {
                let dx = b[i][0] - b[j][0]
                let dy = b[i][1] - b[j][1]
                let dz = b[i][2] - b[j][2]
                e -= b[i][6] * b[j][6] / (dx * dx + dy * dy + dz * dz).squareRoot()
            }
        }
        return q(e, 1e9)
    }

    static func fft(_ n: Int) -> UInt64 {
        var r = XorShift32(state: 0xF00D_F00D)
        var re = (0..<n).map { _ in rnd01(&r) - 0.5 }
        var im = [Double](repeating: 0, count: n)
        var j = 0
        for i in 1..<n {
            var bit = n >> 1
            while j & bit != 0 { j ^= bit; bit >>= 1 }
            j ^= bit
            if i < j { re.swapAt(i, j); im.swapAt(i, j) }
        }
        let half = n / 2
        var twr = [Double](), twi = [Double]()
        twr.reserveCapacity(half); twi.reserveCapacity(half)
        for k in 0..<half {
            let ang = 2.0 * Double.pi * Double(k) / Double(n)
            twr.append(cos(ang))
            twi.append(-sin(ang))
        }
        var len = 2
        while len <= n {
            let step = n / len
            let h = len / 2
            var i = 0
            while i < n {
                for k in 0..<h {
                    let wr = twr[k * step], wi = twi[k * step]
                    let a = i + k, b = a + h
                    let xr = re[b] * wr - im[b] * wi
                    let xi = re[b] * wi + im[b] * wr
                    re[b] = re[a] - xr
                    im[b] = im[a] - xi
                    re[a] += xr
                    im[a] += xi
                }
                i += len
            }
            len <<= 1
        }
        var acc: UInt64 = 0
        for k in 0..<n { acc = acc &+ q(abs(re[k]) + abs(im[k]), 1000.0) }
        return acc
    }

    static func bigintFactorial(_ n: Int) -> UInt64 {
        let base: UInt64 = 1_000_000_000
        var limbs: [UInt32] = [1]
        if n >= 2 {
            for k in 2...UInt64(n) {
                var carry: UInt64 = 0
                for i in limbs.indices {
                    let cur = UInt64(limbs[i]) * k + carry
                    limbs[i] = UInt32(cur % base)
                    carry = cur / base
                }
                while carry > 0 {
                    limbs.append(UInt32(carry % base))
                    carry /= base
                }
            }
        }
        var digits: UInt64 = 0
        for l in limbs {
            var v = l
            while v > 0 { digits += UInt64(v % 10); v /= 10 }
        }
        return digits + (UInt64(limbs.count) << 32)
    }

    static func particles(_ n: Int) -> UInt64 {
        var r = XorShift32(state: 0x0BAD_5EED)
        var ps = [[Double]]()
        ps.reserveCapacity(n)
        for _ in 0..<n {
            let x = rnd01(&r), y = rnd01(&r), vx = rnd01(&r) - 0.5, vy = rnd01(&r) - 0.5
            ps.append([x, y, vx, vy])
        }
        let dt = 0.005
        for k in 0..<n {
            var p = ps[k]
            for _ in 0..<50 {
                p[3] -= 9.81 * dt
                p[0] += p[2] * dt
                p[1] += p[3] * dt
                if p[0] < 0 { p[0] = -p[0]; p[2] = -p[2] * 0.9 } else if p[0] > 1 { p[0] = 2 - p[0]; p[2] = -p[2] * 0.9 }
                if p[1] < 0 { p[1] = -p[1]; p[3] = -p[3] * 0.8 } else if p[1] > 1 { p[1] = 2 - p[1]; p[3] = -p[3] * 0.8 }
                p[2] *= 0.999
                p[3] *= 0.999
            }
            ps[k] = p
        }
        var acc: UInt64 = 0
        for p in ps { acc = acc &+ q(p[0], 1e6) &+ q(p[1], 1e6) }
        return acc
    }

    // MARK: Data

    static func kmeans(_ n: Int) -> UInt64 {
        let k = 16
        var r = XorShift32(state: 0x00C0_FFEE)
        var seeds = [(Double, Double)]()
        for _ in 0..<k { let x = rnd01(&r) * 10.0; let y = rnd01(&r) * 10.0; seeds.append((x, y)) }
        var px = [Double](), py = [Double]()
        px.reserveCapacity(n); py.reserveCapacity(n)
        for _ in 0..<n {
            let c = Int(r.next() % UInt32(k))
            let x = seeds[c].0 + (rnd01(&r) - 0.5) * 1.5
            let y = seeds[c].1 + (rnd01(&r) - 0.5) * 1.5
            px.append(x); py.append(y)
        }
        var cx = Array(px[0..<k]), cy = Array(py[0..<k])
        var assign = [Int](repeating: 0, count: n)
        for _ in 0..<8 {
            for i in 0..<n {
                var best = 0
                var bd = Double.greatestFiniteMagnitude
                for c in 0..<k {
                    let dx = px[i] - cx[c], dy = py[i] - cy[c]
                    let d = dx * dx + dy * dy
                    if d < bd { bd = d; best = c }
                }
                assign[i] = best
            }
            var sx = [Double](repeating: 0, count: k), sy = sx
            var counts = [Int](repeating: 0, count: k)
            for i in 0..<n {
                sx[assign[i]] += px[i]
                sy[assign[i]] += py[i]
                counts[assign[i]] += 1
            }
            for c in 0..<k where counts[c] > 0 {
                cx[c] = sx[c] / Double(counts[c])
                cy[c] = sy[c] / Double(counts[c])
            }
        }
        var acc: UInt64 = 0
        for (i, a) in assign.enumerated() { acc = acc &+ UInt64(a) &* (UInt64(i) % 7 + 1) }
        for c in 0..<k { acc = acc &+ q(cx[c], 1e6) &+ q(cy[c], 1e6) }
        return acc
    }

    static func genText(_ n: Int, _ seed: UInt32) -> [UInt8] {
        var r = XorShift32(state: seed)
        var vocab = [[UInt8]]()
        for _ in 0..<500 {
            let len = 2 + Int(r.next() % 8)
            var w = [UInt8]()
            for _ in 0..<len { w.append(UInt8(97 + r.next() % 26)) }
            vocab.append(w)
        }
        var out = [UInt8]()
        out.reserveCapacity(n + 16)
        while out.count < n {
            let w = vocab[Int(r.next() % 500)]
            let cap = r.next() % 10 == 0
            let start = out.count
            out.append(contentsOf: w)
            if cap { out[start] -= 32 }
            let s = r.next() % 30
            if s == 0 { out.append(46); out.append(10) } else if s < 3 { out.append(44); out.append(32) } else { out.append(32) }
        }
        out.removeLast(out.count - n)
        return out
    }

    static func lzBlock(_ d: ArraySlice<UInt8>) -> UInt64 {
        let hbits: UInt32 = 14
        let d = Array(d)
        let len = d.count
        var head = [Int32](repeating: -1, count: 1 << hbits)
        var prev = [Int32](repeating: -1, count: len)
        func hash(_ p: Int) -> Int {
            let v = UInt32(d[p]) | (UInt32(d[p + 1]) << 8) | (UInt32(d[p + 2]) << 16) | (UInt32(d[p + 3]) << 24)
            return Int((v &* 2_654_435_761) >> (32 - hbits))
        }
        func insert(_ p: Int) {
            if p + 4 <= len {
                let h = hash(p)
                prev[p] = head[h]
                head[h] = Int32(p)
            }
        }
        var acc: UInt64 = 0
        var bits: UInt64 = 0
        var pos = 0
        while pos < len {
            var bestLen = 0, bestDist = 0
            if pos + 4 <= len {
                var cand = head[hash(pos)]
                var depth = 0
                let maxL = min(len - pos, 258)
                while cand >= 0 && depth < 16 && pos - Int(cand) <= 32_768 {
                    let c = Int(cand)
                    var l = 0
                    while l < maxL && d[c + l] == d[pos + l] { l += 1 }
                    if l > bestLen {
                        bestLen = l
                        bestDist = pos - c
                        if l == maxL { break }
                    }
                    cand = prev[c]
                    depth += 1
                }
            }
            if bestLen >= 4 {
                acc = acc &* 31 &+ (UInt64((bestLen << 16) | bestDist) ^ 0x8000_0000_0000)
                bits += 32
                for p in pos..<(pos + bestLen) { insert(p) }
                pos += bestLen
            } else {
                acc = acc &* 31 &+ UInt64(d[pos])
                bits += 9
                insert(pos)
                pos += 1
            }
        }
        return acc &+ (bits << 40)
    }

    static func lz77(_ n: Int) -> UInt64 {
        let block = 1 << 16
        let data = genText(n, 0x0012_3457)
        let blocks = (data.count + block - 1) / block
        var total: UInt64 = 0
        for b in 0..<blocks {
            let end = min((b + 1) * block, data.count)
            total = total &+ lzBlock(data[(b * block)..<end]) &* UInt64(b + 1)
        }
        return total
    }

    // MARK: Text

    static func wordfreq(_ n: Int) -> UInt64 {
        let text = genText(n, 0x0BEE_F123)
        var counts = [[UInt8]: UInt32]()
        var buf = [UInt8]()
        buf.reserveCapacity(16)
        for b in text {
            if (b >= 65 && b <= 90) || (b >= 97 && b <= 122) {
                buf.append(b >= 65 && b <= 90 ? b + 32 : b)
            } else if !buf.isEmpty {
                counts[buf, default: 0] += 1
                buf.removeAll(keepingCapacity: true)
            }
        }
        if !buf.isEmpty { counts[buf, default: 0] += 1 }
        var acc = UInt64(counts.count) << 48
        for (w, c) in counts { acc = acc &+ fnv(w) &* UInt64(c) }
        return acc
    }

    static func reHere(_ re: [UInt8], _ ri: Int, _ t: [UInt8], _ ti: Int) -> Bool {
        if ri == re.count { return true }
        if ri + 1 < re.count && re[ri + 1] == 42 { return reStar(re[ri], re, ri + 2, t, ti) }
        if re[ri] == 36 && ri + 1 == re.count { return ti == t.count }
        if ti < t.count && (re[ri] == 46 || re[ri] == t[ti]) { return reHere(re, ri + 1, t, ti + 1) }
        return false
    }

    static func reStar(_ c: UInt8, _ re: [UInt8], _ ri: Int, _ t: [UInt8], _ ti0: Int) -> Bool {
        var ti = ti0
        while true {
            if reHere(re, ri, t, ti) { return true }
            if ti < t.count && (t[ti] == c || c == 46) { ti += 1 } else { return false }
        }
    }

    static func reMatch(_ re: [UInt8], _ t: [UInt8]) -> Bool {
        if !re.isEmpty && re[0] == 94 { return reHere(re, 1, t, 0) }
        var ti = 0
        while true {
            if reHere(re, 0, t, ti) { return true }
            if ti == t.count { return false }
            ti += 1
        }
    }

    static let patterns: [[UInt8]] = ["ab.*ef", "^b", "q.z", "e*st$", "xa*y"].map { Array($0.utf8) }

    static func textsearch(_ n: Int) -> UInt64 {
        let text = genText(n, 0x7E57_7E57)
        let lines = text.split(separator: 10, omittingEmptySubsequences: false).map { Array($0) }
        var acc: UInt64 = 0
        for (i, l) in lines.enumerated() {
            for (k, p) in patterns.enumerated() where reMatch(p, l) {
                acc = acc &+ UInt64(k + 1) * (UInt64(i) % 3 + 1)
            }
        }
        return acc
    }

    static func strsort(_ n: Int) -> UInt64 {
        var r = XorShift32(state: 0x5747_5747)
        var v = [String]()
        v.reserveCapacity(n)
        for _ in 0..<n {
            let len = 4 + Int(r.next() % 13)
            var bytes = [UInt8]()
            for _ in 0..<len { bytes.append(UInt8(97 + r.next() % 26)) }
            v.append(String(decoding: bytes, as: UTF8.self))
        }
        v.sort()
        var acc = fnv(Array(v[n - 1].utf8))
        for i in stride(from: 0, to: n, by: 97) {
            acc = acc &* 31 &+ fnv(Array(v[i].utf8)) &+ UInt64(i)
        }
        return acc
    }

    static func levPair(_ len: Int, _ pair: Int) -> UInt64 {
        var r = XorShift32(state: 0x5EED + UInt32(pair) * 7919)
        let acgt: [UInt8] = Array("acgt".utf8)
        var a = [UInt8]()
        for _ in 0..<len { a.append(acgt[Int(r.next() % 4)]) }
        var b = [UInt8]()
        for c in a {
            let k = r.next() % 25
            if k == 0 { continue }
            b.append(r.next() % 10 == 0 ? acgt[Int(r.next() % 4)] : c)
            if k == 1 { b.append(acgt[Int(r.next() % 4)]) }
        }
        var prev = (0...b.count).map { UInt32($0) }
        var cur = [UInt32](repeating: 0, count: b.count + 1)
        for (i, ca) in a.enumerated() {
            cur[0] = UInt32(i + 1)
            for (j, cb) in b.enumerated() {
                let sub = prev[j] + (ca != cb ? 1 : 0)
                cur[j + 1] = min(min(sub, prev[j + 1] + 1), cur[j] + 1)
            }
            swap(&prev, &cur)
        }
        return UInt64(prev[b.count]) * UInt64(pair + 1)
    }

    static func levenshtein(_ n: Int) -> UInt64 {
        var acc: UInt64 = 0
        for p in 0..<8 { acc = acc &+ levPair(n, p) }
        return acc
    }

    // MARK: Crypto

    @inline(__always) static func rotl(_ x: UInt32, _ n: UInt32) -> UInt32 { (x << n) | (x >> (32 - n)) }

    @inline(__always) static func qr(_ s: inout [UInt32], _ a: Int, _ b: Int, _ c: Int, _ d: Int) {
        s[a] = s[a] &+ s[b]; s[d] = rotl(s[d] ^ s[a], 16)
        s[c] = s[c] &+ s[d]; s[b] = rotl(s[b] ^ s[c], 12)
        s[a] = s[a] &+ s[b]; s[d] = rotl(s[d] ^ s[a], 8)
        s[c] = s[c] &+ s[d]; s[b] = rotl(s[b] ^ s[c], 7)
    }

    static func chachaBlock(_ key: [UInt32], _ counter: UInt32, _ nonce: [UInt32], _ out: inout [UInt8]) {
        let initial: [UInt32] = [0x6170_7865, 0x3320_646e, 0x7962_2d32, 0x6b20_6574] + key + [counter] + nonce
        var s = initial
        for _ in 0..<10 {
            qr(&s, 0, 4, 8, 12); qr(&s, 1, 5, 9, 13); qr(&s, 2, 6, 10, 14); qr(&s, 3, 7, 11, 15)
            qr(&s, 0, 5, 10, 15); qr(&s, 1, 6, 11, 12); qr(&s, 2, 7, 8, 13); qr(&s, 3, 4, 9, 14)
        }
        for i in 0..<16 {
            let w = s[i] &+ initial[i]
            out[i * 4] = UInt8(truncatingIfNeeded: w)
            out[i * 4 + 1] = UInt8(truncatingIfNeeded: w >> 8)
            out[i * 4 + 2] = UInt8(truncatingIfNeeded: w >> 16)
            out[i * 4 + 3] = UInt8(truncatingIfNeeded: w >> 24)
        }
    }

    static func chacha20(_ n: Int) -> UInt64 {
        let chunk = 4096
        let key: [UInt32] = (0..<8).map { (UInt32($0) &* 0x0101_0101) ^ 0xDEAD_BEEF }
        let nonce: [UInt32] = [0x0000_0009, 0x4a00_0000, 0]
        var data = (0..<n).map { UInt8(truncatingIfNeeded: $0 * 7 + 3) }
        var ks = [UInt8](repeating: 0, count: 64)
        var block = 0
        var off = 0
        while off < n {
            chachaBlock(key, UInt32(block) + 1, nonce, &ks)
            let end = min(off + 64, n)
            for i in off..<end { data[i] ^= ks[i - off] }
            off += 64
            block += 1
        }
        var acc: UInt64 = 0
        var ci = 0
        var start = 0
        while start < n {
            let end = min(start + chunk, n)
            var h: UInt64 = 0
            var p = start
            while p + 8 <= end {
                var w: UInt64 = 0
                for k in 0..<8 { w |= UInt64(data[p + k]) << (8 * UInt64(k)) }
                h = ((h << 5) | (h >> 59)) ^ w
                p += 8
            }
            acc = acc &+ h &* UInt64(ci + 1)
            ci += 1
            start += chunk
        }
        return acc
    }

    static func crc32(_ n: Int) -> UInt64 {
        var table = [UInt32](repeating: 0, count: 256)
        for i in 0..<256 {
            var c = UInt32(i)
            for _ in 0..<8 { c = (c & 1) != 0 ? (0xEDB8_8320 ^ (c >> 1)) : (c >> 1) }
            table[i] = c
        }
        var r = XorShift32(state: 0xC4C3_2C32)
        var data = [UInt8]()
        data.reserveCapacity(n)
        for _ in 0..<n { data.append(UInt8(truncatingIfNeeded: r.next())) }
        var crc: UInt32 = ~0
        for b in data { crc = table[Int((crc ^ UInt32(b)) & 0xff)] ^ (crc >> 8) }
        return UInt64(~crc) | (UInt64(n) << 32)
    }

    // MARK: Algorithms

    /// Minimal binary min-heap (the Swift standard library has no priority queue).
    struct MinHeap {
        var a: [(UInt32, UInt32)] = []
        var isEmpty: Bool { a.isEmpty }
        @inline(__always) static func less(_ x: (UInt32, UInt32), _ y: (UInt32, UInt32)) -> Bool {
            x.0 < y.0 || (x.0 == y.0 && x.1 < y.1)
        }
        mutating func push(_ v: (UInt32, UInt32)) {
            a.append(v)
            var i = a.count - 1
            while i > 0 {
                let p = (i - 1) / 2
                if MinHeap.less(a[i], a[p]) { a.swapAt(i, p); i = p } else { break }
            }
        }
        mutating func pop() -> (UInt32, UInt32)? {
            guard !a.isEmpty else { return nil }
            let top = a[0]
            let last = a.removeLast()
            if !a.isEmpty {
                a[0] = last
                var i = 0
                while true {
                    let l = 2 * i + 1, r = l + 1
                    var m = i
                    if l < a.count && MinHeap.less(a[l], a[m]) { m = l }
                    if r < a.count && MinHeap.less(a[r], a[m]) { m = r }
                    if m == i { break }
                    a.swapAt(i, m)
                    i = m
                }
            }
            return top
        }
    }

    static func dijkstra(_ n: Int) -> UInt64 {
        var r = XorShift32(state: 0xD1D1_D1D1)
        var cost = [UInt32]()
        cost.reserveCapacity(n * n)
        for _ in 0..<(n * n) { cost.append(1 + r.next() % 16) }
        var dist = [UInt32](repeating: .max, count: n * n)
        var heap = MinHeap()
        dist[0] = 0
        heap.push((0, 0))
        while let (d, uu) = heap.pop() {
            let u = Int(uu)
            if d > dist[u] { continue }
            let x = u % n, y = u / n
            func relax(_ v: Int) {
                let nd = d + cost[v]
                if nd < dist[v] { dist[v] = nd; heap.push((nd, UInt32(v))) }
            }
            if x > 0 { relax(u - 1) }
            if x + 1 < n { relax(u + 1) }
            if y > 0 { relax(u - n) }
            if y + 1 < n { relax(u + n) }
        }
        var sum: UInt64 = 0
        var mx: UInt32 = 0
        for d in dist { sum += UInt64(d); if d > mx { mx = d } }
        return sum &+ (UInt64(mx) << 40)
    }

    static func bfs(_ n: Int) -> UInt64 {
        var r = XorShift32(state: 0xBF5B_F5BF)
        let m = n * 4
        var eu = [Int](), ev = [Int]()
        eu.reserveCapacity(m); ev.reserveCapacity(m)
        for _ in 0..<m {
            let u = Int(r.next()) % n
            let v = Int(r.next()) % n
            eu.append(u); ev.append(v)
        }
        var off = [Int](repeating: 0, count: n + 1)
        for e in 0..<m { off[eu[e] + 1] += 1; off[ev[e] + 1] += 1 }
        for i in 0..<n { off[i + 1] += off[i] }
        var fill = off
        var adj = [Int](repeating: 0, count: m * 2)
        for e in 0..<m {
            let u = eu[e], v = ev[e]
            adj[fill[u]] = v; fill[u] += 1
            adj[fill[v]] = u; fill[v] += 1
        }
        var level = [UInt32](repeating: .max, count: n)
        var queue = [Int]()
        queue.reserveCapacity(n)
        level[0] = 0
        queue.append(0)
        var head = 0
        while head < queue.count {
            let u = queue[head]
            head += 1
            for k in off[u]..<off[u + 1] {
                let v = adj[k]
                if level[v] == .max { level[v] = level[u] + 1; queue.append(v) }
            }
        }
        var acc = UInt64(queue.count) << 40
        for (i, l) in level.enumerated() where l != .max {
            acc = acc &+ UInt64(l) &* (UInt64(i) % 5 + 1)
        }
        return acc
    }

    static func queens(_ mask: UInt32, _ cols: UInt32, _ d1: UInt32, _ d2: UInt32) -> UInt64 {
        if cols == mask { return 1 }
        var avail = ~(cols | d1 | d2) & mask
        var count: UInt64 = 0
        while avail != 0 {
            let bit = avail & (0 &- avail)
            avail ^= bit
            count += queens(mask, cols | bit, (d1 | bit) << 1, (d2 | bit) >> 1)
        }
        return count
    }

    static func nqueens(_ n: Int) -> UInt64 {
        let mask: UInt32 = (1 << UInt32(n)) - 1
        var total: UInt64 = 0
        for c in 0..<n {
            let bit: UInt32 = 1 << UInt32(c)
            total += queens(mask, bit, bit << 1, bit >> 1)
        }
        return total
    }

    static let puzzles: [String] = [
        "4.....8.5.3..........7......2.....6.....8.4......1.......6.3.7.5..2.....1.4......",
        "52...6.........7.13...........4..8..6......5...........418.........3..2...87.....",
        "85...24..72......9..4.........1.7..23.5...9...4...........8..7..17..........36.4.",
        "..53.....8......2..7..1.5..4....53...1..7...6..32...8..6.5....9..4....3......97..",
    ]

    struct Sudoku {
        var g = [UInt8](repeating: 0, count: 81)
        var rows = [UInt16](repeating: 0, count: 9)
        var cols = [UInt16](repeating: 0, count: 9)
        var boxes = [UInt16](repeating: 0, count: 9)
        var nodes: UInt64 = 0

        init(_ p: String) {
            for (i, ch) in p.utf8.enumerated() where ch >= 49 && ch <= 57 { set(i, ch - 48) }
        }
        mutating func set(_ i: Int, _ d: UInt8) {
            let bit: UInt16 = 1 << UInt16(d)
            g[i] = d
            rows[i / 9] |= bit
            cols[i % 9] |= bit
            boxes[(i / 27) * 3 + (i % 9) / 3] |= bit
        }
        mutating func unset(_ i: Int, _ d: UInt8) {
            let bit: UInt16 = ~(1 << UInt16(d))
            g[i] = 0
            rows[i / 9] &= bit
            cols[i % 9] &= bit
            boxes[(i / 27) * 3 + (i % 9) / 3] &= bit
        }
        mutating func solve() -> Bool {
            nodes += 1
            var best = -1
            var bestCnt = 10
            var bestMask: UInt16 = 0
            for i in 0..<81 where g[i] == 0 {
                let used = rows[i / 9] | cols[i % 9] | boxes[(i / 27) * 3 + (i % 9) / 3]
                let free = ~used & 0x3FE
                let cnt = free.nonzeroBitCount
                if cnt < bestCnt {
                    best = i; bestCnt = cnt; bestMask = free
                    if cnt <= 1 { break }
                }
            }
            if best < 0 { return true }
            for d in UInt8(1)...9 where bestMask & (1 << UInt16(d)) != 0 {
                set(best, d)
                if solve() { return true }
                unset(best, d)
            }
            return false
        }
    }

    static func sudoku(_ n: Int) -> UInt64 {
        var total: UInt64 = 0
        for k in 0..<(n * puzzles.count) {
            var s = Sudoku(puzzles[k % puzzles.count])
            let ok = s.solve()
            var grid: UInt64 = 0
            for (i, d) in s.g.enumerated() { grid += UInt64(d) * UInt64(i + 1) }
            total = total &+ (grid + (s.nodes << 20) + (ok ? 1 : 0))
        }
        return total
    }
}
