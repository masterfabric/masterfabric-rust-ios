# MasterFabric Rust (iOS)

A shared Rust core with a native SwiftUI UI. State, navigation, data, business rules and
benchmark orchestration live in Rust (`crates/app-core`), exposed to Swift through UniFFI
(`crates/app-ffi`). SwiftUI only renders and forwards user events.

```
SwiftUI view ──dispatch(Action)──▶ Rust reducer ──dirty mask──▶ partial ViewUpdate ──▶ SwiftUI
```

## Screens
- **Home**: MasterFabric textmark header, searchable project list (search/sort/favorites in Rust).
- **Detail**: item detail, favorites, related items (Rust `DetailViewModel`).
- **Benchmark**: Rust vs Swift, same algorithm and data, checksums compared. Media workloads first,
  ray-trace output rendered from the Rust RGBA buffer next to the Swift one, results table,
  Swift Charts (ratio + duration), app performance card (launch, dispatch→apply, dispatch→frame).
- **Info**: what you can build with Rust, architecture diagram, ratios measured by this app only.
  All content is served from Rust.

## Benchmarks
| Category | Test | Rust ∥ (rayon) |
|---|---|---|
| Media | Ray tracing 256², triangle rasterization (3000 tris, z-buffer) | ✓ |
| Media | Image pipeline 1024² (gaussian blur, bilinear resize, color grade LUT, RGB→YUV) | ✓ |
| Media | Video: 30 synthetic 640×360 frames (YUV, blur, frame diff, 16×16 block motion estimation) | ✓ |
| Media | 3D mesh: icosphere, normals, OBJ export | – |
| General | fibonacci, sort 1M, SHA-256 (hand-written), JSON, sieve, matmul, HashMap | sort, matmul |

Each test: 1 warm-up + N measured runs on a background thread. Ratio = Swift avg / Rust
single-thread avg. Rayon variants reuse the same row/band kernels (identical checksums) and are
shown separately, never mixed into the ratio.

## Performance work
- `[profile.release]`: `opt-level=3`, `lto="fat"`, `codegen-units=1`, `panic="abort"`, `strip=true`.
  Trade-off: with `panic=abort` a Rust panic aborts the process instead of being turned into a
  Swift error by UniFFI's `catch_unwind`.
- `.cargo/config.toml`: `target-cpu=apple-m1` for the simulator, `apple-a14` for devices.
  Note: the Swift side is not CPU-tuned the same way, so this slightly favours Rust.
- FFI: `dispatch` returns a dirty mask; only changed slices (`stack`, `home`, `details`,
  `benchmark`, `info`, `perf`) are sent as a `ViewUpdate`. No-op actions emit nothing.
  Large buffers (ray-trace image) cross once as `Vec<u8>`. Swift keeps each slice in its own
  `@Observable` property so unrelated views are not invalidated.
- Hot loops: slice/zip iteration, `as_chunks`/`chunks_exact`, no allocations inside loops.
- Measured in-app and recorded into Rust via `Action::RecordPerf`: launch → first frame,
  Rust core init, dispatch → state applied (avg/p95), full snapshot cost, dispatch → next frame.

## Build & run
```sh
./scripts/build-ios.sh        # Rust release (device + sim), UniFFI bindings, xcframework, xcodegen
./scripts/run-sim.sh          # build, install and launch on the booted simulator (default iPhone 18 Pro)
./scripts/measure.sh after 3  # N launches with -probe/-autorun, medians from build/perf/*.json
cargo test --release && cargo clippy --workspace --all-targets --release
```
Launch arguments: `-startScreen home|detail|benchmark|info`, `-detailId N`, `-autorun YES`,
`-probe YES`, `-scrollTo media|table|charts|perf|<bench id>`, `-infoScrollTo projects|measured|perf`, `-query text`.

## Before / after
`build/perf/before-*.json` vs `build/perf/after-*.json` (medians of 3, iPhone simulator on an M5 MacBook Air).
The "after" run was taken while the Mac was on battery at 2–3 % and throttled (unchanged
Swift code also got ~3–4× slower), so the absolute numbers are not comparable; re-run
`./scripts/measure.sh after 3` on power for a fair comparison.

Limits: simulator numbers measure the Mac CPU, not an iPhone. Swift has no parallel variants.
