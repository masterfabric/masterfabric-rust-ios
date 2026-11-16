# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| `main` (latest) | ✅ |
| older commits | ❌ |

## Reporting a Vulnerability

**Do not open a public issue for security vulnerabilities.**

Report it privately through GitHub's private vulnerability reporting for this repository:
**[Report a vulnerability](https://github.com/masterfabric/masterfabric-rust-ios/security/advisories/new)** (Security tab → "Report a vulnerability").
Include:

- a description of the vulnerability and its impact,
- steps to reproduce (device / simulator, iOS version, commit),
- a suggested fix, if you have one,
- how we can contact you.

For critical issues start the advisory title with `[URGENT]`.

### Response timeline

- **Initial response:** within 48 hours
- **Assessment / status update:** within 7 days
- **Fix:** prioritized and released as soon as possible

Researchers are credited in the release notes unless they prefer to stay anonymous.

## Scope

This repository is a demo app: a native SwiftUI UI on a shared Rust core (UniFFI). It has no
network access, no accounts and no backend. Relevant areas include:

- memory safety at the FFI boundary (`crates/app-ffi`, generated UniFFI bindings),
- `unsafe` code in the Rust crates,
- the build scripts in `scripts/`,
- third-party dependencies (please also report those upstream).

Not in scope: theoretical issues without a proof of concept, social engineering, physical
attacks, and automated scanner output without analysis.

## Notes for contributors

- Avoid `unsafe` Rust; if it is required, document the invariants next to it.
- Keep dependencies minimal and up to date (`cargo update`, review `Cargo.lock` diffs).
- Never commit signing identities, provisioning profiles, API keys or other secrets.
- Release builds use `panic = "abort"`: a Rust panic terminates the app instead of crossing the
  FFI boundary.

## Legal

Copyright (c) 2026 MasterFabric. Licensed under the GNU AGPL v3.0, see [LICENSE](LICENSE) and
[NOTICE](NOTICE). Security fixes and contributions are licensed under the same terms.
