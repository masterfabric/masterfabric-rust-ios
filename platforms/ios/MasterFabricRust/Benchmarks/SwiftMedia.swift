import Foundation

// One-to-one Swift port of `crates/app-core/src/media.rs`.
// Pure CPU, idiomatic Swift (Array, struct), no system libraries
// (Foundation `String(format:)` is used for the OBJ text).

@inline(__always) private func hashStep(_ acc: UInt64, _ b: UInt64) -> UInt64 { acc &* 31 &+ b }

enum SwiftMedia {
    // MARK: Ray tracer

    struct V3 {
        var x, y, z: Double
        @inline(__always) static func + (a: V3, b: V3) -> V3 { V3(x: a.x + b.x, y: a.y + b.y, z: a.z + b.z) }
        @inline(__always) static func - (a: V3, b: V3) -> V3 { V3(x: a.x - b.x, y: a.y - b.y, z: a.z - b.z) }
        @inline(__always) static func * (a: V3, s: Double) -> V3 { V3(x: a.x * s, y: a.y * s, z: a.z * s) }
        @inline(__always) func dot(_ o: V3) -> Double { x * o.x + y * o.y + z * o.z }
        @inline(__always) func norm() -> V3 { let l = dot(self).squareRoot(); return V3(x: x / l, y: y / l, z: z / l) }
    }

    struct Sphere { let c: V3; let r: Double; let color: V3; let reflect: Double }

    static let spheres: [Sphere] = [
        Sphere(c: V3(x: 0, y: 0, z: -3), r: 0.5, color: V3(x: 0.9, y: 0.2, z: 0.2), reflect: 0.3),
        Sphere(c: V3(x: 1.1, y: 0, z: -3.5), r: 0.5, color: V3(x: 0.2, y: 0.9, z: 0.3), reflect: 0.3),
        Sphere(c: V3(x: -1.1, y: 0, z: -3.5), r: 0.5, color: V3(x: 0.2, y: 0.4, z: 0.9), reflect: 0.3),
        Sphere(c: V3(x: 0, y: 0.9, z: -4), r: 0.4, color: V3(x: 0.95, y: 0.85, z: 0.2), reflect: 0.5),
    ]
    static let planeY = -0.5
    static let eps = 1e-4

    @inline(__always) static func hitSphere(_ s: Sphere, _ o: V3, _ d: V3) -> Double? {
        let oc = o - s.c
        let b = oc.dot(d)
        let c = oc.dot(oc) - s.r * s.r
        let disc = b * b - c
        if disc < 0 { return nil }
        let sq = disc.squareRoot()
        let t0 = -b - sq
        if t0 > eps { return t0 }
        let t1 = -b + sq
        return t1 > eps ? t1 : nil
    }

    static func trace(_ o: V3, _ d: V3, _ depth: Int, _ light: V3) -> V3 {
        var bestT = Double.greatestFiniteMagnitude
        var best: Int? = nil
        for i in 0..<spheres.count {
            if let t = hitSphere(spheres[i], o, d), t < bestT { bestT = t; best = i }
        }
        var plane = false
        if d.y < -1e-9 {
            let t = (planeY - o.y) / d.y
            if t > eps && t < bestT { bestT = t; plane = true; best = nil }
        }
        if best == nil && !plane {
            let k = 0.5 * (d.y + 1)
            return V3(x: 1, y: 1, z: 1) * (1 - k) + V3(x: 0.5, y: 0.7, z: 1) * k
        }
        let p = o + d * bestT
        let n: V3, color: V3, reflect: Double
        if let i = best {
            let s = spheres[i]
            n = (p - s.c) * (1 / s.r); color = s.color; reflect = s.reflect
        } else {
            let parity = (Int64(p.x.rounded(.down)) + Int64(p.z.rounded(.down))) & 1
            let g = parity == 0 ? 0.9 : 0.3
            n = V3(x: 0, y: 1, z: 0); color = V3(x: g, y: g, z: g); reflect = 0.2
        }
        var diffuse = max(n.dot(light), 0)
        let sp = p + n * eps
        if spheres.contains(where: { hitSphere($0, sp, light) != nil }) { diffuse *= 0.3 }
        var col = color * (0.1 + 0.9 * diffuse)
        if depth < 2 && reflect > 0 {
            let rd = d - n * (2 * d.dot(n))
            col = col * (1 - reflect) + trace(sp, rd, depth + 1, light) * reflect
        }
        return col
    }

    static func raytrace(_ w: Int, _ h: Int) -> [UInt8] {
        let light = V3(x: -0.6, y: 1, z: 0.5).norm()
        let origin = V3(x: 0, y: 0.2, z: 0.5)
        let offsets: [(Double, Double)] = [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)]
        var out = [UInt8](repeating: 0, count: w * h * 4)
        for y in 0..<h {
            for x in 0..<w {
                var acc = V3(x: 0, y: 0, z: 0)
                for (ox, oy) in offsets {
                    let u = ((Double(x) + ox) / Double(w)) * 2 - 1
                    let v = 1 - ((Double(y) + oy) / Double(h)) * 2
                    acc = acc + trace(origin, V3(x: u, y: v, z: -1.5).norm(), 0, light)
                }
                let c = acc * 0.25
                let i = (y * w + x) * 4
                out[i] = UInt8(min(max(c.x, 0), 1).squareRoot() * 255)
                out[i + 1] = UInt8(min(max(c.y, 0), 1).squareRoot() * 255)
                out[i + 2] = UInt8(min(max(c.z, 0), 1).squareRoot() * 255)
                out[i + 3] = 255
            }
        }
        return out
    }

    static func raytraceBench(_ size: Int) -> UInt64 {
        raytrace(size, size).reduce(0) { hashStep($0, UInt64($1)) }
    }

    // MARK: Rasterizer

    @inline(__always) static func edge(_ ax: Int32, _ ay: Int32, _ bx: Int32, _ by: Int32, _ px: Int32, _ py: Int32) -> Int32 {
        (bx - ax) * (py - ay) - (by - ay) * (px - ax)
    }

    static func rasterize(_ n: Int) -> UInt64 {
        let W: Int32 = 512, H: Int32 = 512
        var color = [UInt32](repeating: 0, count: Int(W * H))
        var depth = [Int32](repeating: Int32.max, count: Int(W * H))
        var rng = XorShift32(state: 0x0BAD_C0DE)
        for _ in 0..<n {
            let cx = Int32(rng.next() % UInt32(W))
            let cy = Int32(rng.next() % UInt32(H))
            let x0 = cx + Int32(rng.next() % 64) - 32
            let y0 = cy + Int32(rng.next() % 64) - 32
            var x1 = cx + Int32(rng.next() % 64) - 32
            var y1 = cy + Int32(rng.next() % 64) - 32
            var x2 = cx + Int32(rng.next() % 64) - 32
            var y2 = cy + Int32(rng.next() % 64) - 32
            let z = Int32(rng.next() % 10_000)
            let c = rng.next()
            let area = edge(x0, y0, x1, y1, x2, y2)
            if area == 0 { continue }
            if area < 0 { swap(&x1, &x2); swap(&y1, &y2) }
            let minx = max(min(x0, x1, x2), 0), maxx = min(max(x0, x1, x2), W - 1)
            let miny = max(min(y0, y1, y2), 0), maxy = min(max(y0, y1, y2), H - 1)
            if minx > maxx || miny > maxy { continue }
            for py in miny...maxy {
                for px in minx...maxx {
                    let w0 = edge(x1, y1, x2, y2, px, py)
                    let w1 = edge(x2, y2, x0, y0, px, py)
                    let w2 = edge(x0, y0, x1, y1, px, py)
                    if w0 >= 0 && w1 >= 0 && w2 >= 0 {
                        let i = Int(py * W + px)
                        if z < depth[i] { depth[i] = z; color[i] = c }
                    }
                }
            }
        }
        return color.reduce(0) { hashStep($0, UInt64($1)) }
    }

    // MARK: Image processing

    static func imagePipeline(_ size: Int) -> UInt64 {
        let w = size, h = size
        var src = [UInt8](repeating: 0, count: w * h * 3)
        for y in 0..<h {
            for x in 0..<w {
                let i = (y * w + x) * 3
                src[i] = UInt8((x ^ y) & 0xff)
                src[i + 1] = UInt8((x * 2 + y) & 0xff)
                src[i + 2] = UInt8(((x * y) >> 4) & 0xff)
            }
        }
        let k: [UInt32] = [1, 4, 6, 4, 1]
        var tmp = [UInt8](repeating: 0, count: w * h * 3)
        for y in 0..<h {
            for x in 0..<w {
                for ch in 0..<3 {
                    var s: UInt32 = 0
                    for t in 0..<5 {
                        let xx = min(max(x + t - 2, 0), w - 1)
                        s += k[t] * UInt32(src[(y * w + xx) * 3 + ch])
                    }
                    tmp[(y * w + x) * 3 + ch] = UInt8((s + 8) >> 4)
                }
            }
        }
        var blur = [UInt8](repeating: 0, count: w * h * 3)
        for y in 0..<h {
            for x in 0..<w {
                for ch in 0..<3 {
                    var s: UInt32 = 0
                    for t in 0..<5 {
                        let yy = min(max(y + t - 2, 0), h - 1)
                        s += k[t] * UInt32(tmp[(yy * w + x) * 3 + ch])
                    }
                    blur[(y * w + x) * 3 + ch] = UInt8((s + 8) >> 4)
                }
            }
        }
        let dw = w / 2, dh = h / 2
        var small = [UInt8](repeating: 0, count: dw * dh * 3)
        for y in 0..<dh {
            let sy = (UInt64(y) * (UInt64(h - 1) << 16)) / UInt64(dh - 1)
            let y0 = Int(sy >> 16), fy = sy & 0xffff, y1 = min(y0 + 1, h - 1)
            for x in 0..<dw {
                let sx = (UInt64(x) * (UInt64(w - 1) << 16)) / UInt64(dw - 1)
                let x0 = Int(sx >> 16), fx = sx & 0xffff, x1 = min(x0 + 1, w - 1)
                for ch in 0..<3 {
                    let p00 = UInt64(blur[(y0 * w + x0) * 3 + ch]), p10 = UInt64(blur[(y0 * w + x1) * 3 + ch])
                    let p01 = UInt64(blur[(y1 * w + x0) * 3 + ch]), p11 = UInt64(blur[(y1 * w + x1) * 3 + ch])
                    let top = p00 * (65536 - fx) + p10 * fx
                    let bot = p01 * (65536 - fx) + p11 * fx
                    small[(y * dw + x) * 3 + ch] = UInt8((top * (65536 - fy) + bot * fy) >> 32)
                }
            }
        }
        var lut = [UInt8](repeating: 0, count: 256)
        for i in 0..<256 { lut[i] = UInt8(min(max(((i - 128) * 6 / 5) + 138, 0), 255)) }
        var sum: UInt64 = 0
        var i = 0
        while i < small.count {
            let r = Int32(lut[Int(small[i])]), g = Int32(lut[Int(small[i + 1])]), b = Int32(lut[Int(small[i + 2])])
            let yv = ((66 * r + 129 * g + 25 * b + 128) >> 8) + 16
            let u = ((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128
            let v = ((112 * r - 94 * g - 18 * b + 128) >> 8) + 128
            sum += UInt64(yv) + 3 * UInt64(u) + 7 * UInt64(v)
            i += 3
        }
        return sum
    }

    // MARK: Video processing

    static func videoPipeline(_ frames: Int) -> UInt64 {
        let W = 640, H = 360, B = 16
        var luma = [UInt8](repeating: 0, count: W * H)
        var cur = [UInt8](repeating: 0, count: W * H)
        var prev = [UInt8](repeating: 0, count: W * H)
        var hasPrev = false
        var diffTotal: UInt64 = 0, sadTotal: UInt64 = 0
        var mvx: Int64 = 0, mvy: Int64 = 0
        for f in 0..<frames {
            let sqx = 40 + f * 6, sqy = 60 + f * 3
            for y in 0..<H {
                for x in 0..<W {
                    let r: UInt32, g: UInt32, b: UInt32
                    if x >= sqx && x < sqx + 80 && y >= sqy && y < sqy + 80 {
                        r = 240; g = 240; b = 240
                    } else {
                        r = UInt32((x + f * 4) & 0xff); g = UInt32((y * 2 + f * 2) & 0xff); b = UInt32(((x + y) / 2) & 0xff)
                    }
                    luma[y * W + x] = UInt8(((66 * r + 129 * g + 25 * b + 128) >> 8) + 16)
                }
            }
            for y in 0..<H {
                for x in 0..<W {
                    if y == 0 || x == 0 || y == H - 1 || x == W - 1 {
                        cur[y * W + x] = luma[y * W + x]
                    } else {
                        var s: UInt32 = 0
                        for dy in 0..<3 { for dx in 0..<3 { s += UInt32(luma[(y + dy - 1) * W + (x + dx - 1)]) } }
                        cur[y * W + x] = UInt8(s / 9)
                    }
                }
            }
            if hasPrev {
                for i in 0..<(W * H) { diffTotal += UInt64(abs(Int32(cur[i]) - Int32(prev[i]))) }
                for by in 0..<(H / B) {
                    for bx in 0..<(W / B) {
                        let ox = bx * B, oy = by * B
                        var best = UInt64.max
                        var bdx = 0, bdy = 0
                        for dy in -2...2 {
                            for dx in -2...2 {
                                let px = ox + dx, py = oy + dy
                                if px < 0 || py < 0 || px + B > W || py + B > H { continue }
                                var sad: UInt64 = 0
                                for yy in 0..<B {
                                    let crow = (oy + yy) * W + ox, prow = (py + yy) * W + px
                                    for xx in 0..<B { sad += UInt64(abs(Int32(cur[crow + xx]) - Int32(prev[prow + xx]))) }
                                }
                                if sad < best { best = sad; bdx = dx; bdy = dy }
                            }
                        }
                        sadTotal += best
                        mvx += Int64(bdx); mvy += Int64(bdy)
                    }
                }
            }
            swap(&cur, &prev)
            hasPrev = true
        }
        return hashStep(hashStep(hashStep(diffTotal, sadTotal), UInt64(mvx + 100_000)), UInt64(mvy + 100_000))
    }

    // MARK: 3D mesh generation

    typealias P3 = (Double, Double, Double)

    @inline(__always) static func normalize(_ v: P3) -> P3 {
        let l = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).squareRoot()
        return (v.0 / l, v.1 / l, v.2 / l)
    }

    static func generateMesh(_ levels: Int) -> (vertices: Int, triangles: Int, objBytes: Int, checksum: UInt64) {
        let t = (1.0 + 5.0.squareRoot()) / 2.0
        var verts: [P3] = [
            (-1, t, 0), (1, t, 0), (-1, -t, 0), (1, -t, 0),
            (0, -1, t), (0, 1, t), (0, -1, -t), (0, 1, -t),
            (t, 0, -1), (t, 0, 1), (-t, 0, -1), (-t, 0, 1),
        ].map(normalize)
        var faces: [(UInt32, UInt32, UInt32)] = [
            (0, 11, 5), (0, 5, 1), (0, 1, 7), (0, 7, 10), (0, 10, 11),
            (1, 5, 9), (5, 11, 4), (11, 10, 2), (10, 7, 6), (7, 1, 8),
            (3, 9, 4), (3, 4, 2), (3, 2, 6), (3, 6, 8), (3, 8, 9),
            (4, 9, 5), (2, 4, 11), (6, 2, 10), (8, 6, 7), (9, 8, 1),
        ]
        for _ in 0..<levels {
            var cache = [UInt64: UInt32]()
            func midpoint(_ a: UInt32, _ b: UInt32) -> UInt32 {
                let key = (UInt64(min(a, b)) << 32) | UInt64(max(a, b))
                if let i = cache[key] { return i }
                let p = verts[Int(a)], q = verts[Int(b)]
                verts.append(normalize(((p.0 + q.0) * 0.5, (p.1 + q.1) * 0.5, (p.2 + q.2) * 0.5)))
                let i = UInt32(verts.count - 1)
                cache[key] = i
                return i
            }
            var next: [(UInt32, UInt32, UInt32)] = []
            next.reserveCapacity(faces.count * 4)
            for f in faces {
                let a = midpoint(f.0, f.1), b = midpoint(f.1, f.2), c = midpoint(f.2, f.0)
                next.append((f.0, a, c)); next.append((f.1, b, a)); next.append((f.2, c, b)); next.append((a, b, c))
            }
            faces = next
        }
        for i in 0..<verts.count {
            let (x, y, z) = verts[i]
            let d = 1.0 + 0.08 * sin(5.0 * x) * sin(5.0 * y) * sin(5.0 * z) + 0.03 * sin(13.0 * x + 7.0 * y)
            verts[i] = (x * d, y * d, z * d)
        }
        var normals = [P3](repeating: (0, 0, 0), count: verts.count)
        for f in faces {
            let a = verts[Int(f.0)], b = verts[Int(f.1)], c = verts[Int(f.2)]
            let u = (b.0 - a.0, b.1 - a.1, b.2 - a.2), v = (c.0 - a.0, c.1 - a.1, c.2 - a.2)
            let n = (u.1 * v.2 - u.2 * v.1, u.2 * v.0 - u.0 * v.2, u.0 * v.1 - u.1 * v.0)
            for i in [f.0, f.1, f.2] {
                let s = normals[Int(i)]
                normals[Int(i)] = (s.0 + n.0, s.1 + n.1, s.2 + n.2)
            }
        }
        normals = normals.map(normalize)
        var obj = ""
        obj.reserveCapacity(verts.count * 64 + faces.count * 32)
        for v in verts { obj += String(format: "v %.4f %.4f %.4f\n", v.0, v.1, v.2) }
        for n in normals { obj += String(format: "vn %.4f %.4f %.4f\n", n.0, n.1, n.2) }
        for f in faces {
            let a = f.0 + 1, b = f.1 + 1, c = f.2 + 1
            obj += "f \(a)//\(a) \(b)//\(b) \(c)//\(c)\n"
        }
        var q: Int64 = 0
        for i in 0..<verts.count {
            let v = verts[i], n = normals[i]
            q = q &+ (Int64((v.0 * 1e6).rounded()) + Int64((v.1 * 1e6).rounded()) + Int64((v.2 * 1e6).rounded()))
            q = q &+ (Int64((n.0 * 1e3).rounded()) + Int64((n.1 * 1e3).rounded()) + Int64((n.2 * 1e3).rounded()))
        }
        let checksum = hashStep(hashStep(UInt64(verts.count), UInt64(faces.count)), UInt64(bitPattern: q))
        return (verts.count, faces.count, obj.utf8.count, checksum)
    }
}
