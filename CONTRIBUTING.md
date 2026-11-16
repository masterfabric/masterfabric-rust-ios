# Contributing to MasterFabric Rust (iOS)

Thanks for your interest! This repo is a native SwiftUI app on a shared Rust core
(UniFFI). Contributions of all kinds are welcome: bug reports, new benchmarks, UI polish,
performance work and documentation.

## Ways to contribute

- **Report bugs** and **request features** through the issue templates.
- **Add a benchmark**: a Rust kernel plus a Swift twin that produce the same checksum.
- **Improve the UI** (SwiftUI, light mode first, no overlapping content).
- **Improve docs**: README flow, build steps, comments.
- **Security issues**: never open a public issue, see [SECURITY.md](SECURITY.md).

## Development setup

### Prerequisites

```bash
# Required
xcode-select --install                       # plus a recent Xcode with the iOS simulator
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
brew install xcodegen
```

### Quick start

```bash
./scripts/build-ios.sh            # Rust release libs, UniFFI Swift bindings, xcframework, xcodegen
./scripts/run-sim.sh              # Release build, install and launch on the simulator
cargo test --release
```

Generated files (`platforms/ios/MasterFabricRust/Generated/`, the `.xcodeproj`, the
xcframework) are build output and gitignored; edit `platforms/ios/project.yml` instead of the
Xcode project.

### Project structure

| Path | What |
|---|---|
| `crates/app-core` | state, reducer (`dispatch(Action)` → partial `ViewUpdate`), view models, benchmark kernels |
| `crates/app-ffi` | UniFFI surface (`AppCore`, `StateListener`, `SwiftBenchRunner`) |
| `platforms/ios/MasterFabricRust` | SwiftUI app (`Bridge/`, `Screens/`, `Benchmarks/`, `Brand/`) |
| `platforms/ios/MasterFabricRustUITests` | XCUITest flow tests |
| `scripts/` | build, run, measure and crosscheck scripts |

## Contribution workflow

### 1. Fork and branch

```bash
git checkout -b feature/your-feature-name
git checkout -b fix/issue-description
git checkout -b docs/what-you-document
```

### 2. Make changes

**Architecture rules**
- Business state lives in Rust. SwiftUI only renders and sends `Action`s.
- Keep `ViewUpdate`s partial: mark only the dirty slices.
- Long work (benchmarks) runs off the main thread.

**Rust**
- `cargo fmt --all`, `cargo clippy --workspace --all-targets --release` with 0 warnings.
- No allocations in hot loops; avoid `unsafe`.

**Swift**
- SwiftUI with `@Observable`; no business logic in views.
- Parenthesize mixed bit/shift/arithmetic expressions (Swift precedence differs from Rust).

**Benchmarks**
- Same input generator (XorShift32), same algorithm and same checksum in both languages.
- Quantize floating-point values per item before summing so checksums do not depend on order
  (needed for rayon variants).
- Add the test to `screens/benchmark.rs`, the Swift twin to `Benchmarks/`, and run
  `./scripts/crosscheck.sh` (must report 0 mismatches).

**Commit message format**

```bash
# Format: type: description
feat: add LZ4 benchmark with Swift twin
fix: correct checksum in Lanczos resize
docs: update launch arguments table
perf: remove allocation from blur inner loop
test: add flow test for category run
refactor: split heavy.rs by category
```

### 3. Test

```bash
cargo test --release
./scripts/crosscheck.sh
xcodebuild test -project platforms/ios/MasterFabricRust.xcodeproj -scheme MasterFabricRust \
  -destination 'platform=iOS Simulator,name=iPhone 18 Pro'
```

### 4. Submit a pull request

Push your branch, open a PR against `main` and fill in the PR template (include simulator
screenshots for UI changes).

## Code review

- At least one maintainer review before merge.
- CI-equivalent checks above must pass.
- Benchmark changes must show before/after numbers and checksum parity.

## Community

Please follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## Legal

### License
Copyright (c) 2026 MasterFabric. The project is licensed under the GNU AGPL v3.0
([LICENSE](LICENSE)) with the attribution and trademark terms in [NOTICE](NOTICE).
By contributing, you agree that your contributions are licensed under the same terms.

### Contributor agreement
By submitting a pull request you confirm that you have the right to license your contribution,
that it is your original work (or properly attributed), and that you license it under the terms
above.

## Questions?

- 💬 [GitHub Issues](https://github.com/masterfabric/masterfabric-rust-ios/issues) (use the Question template)
- 🔒 Security: [private vulnerability report](https://github.com/masterfabric/masterfabric-rust-ios/security/advisories/new)
