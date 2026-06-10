# Changelog

All notable changes to the published `fountain_raptor_10` crate are documented here.

## [1.2.0] - 2026-06-08

### Changed

- **Dependencies** bumped to `fountain_engine` **1.3.1**, `fountain_scheme` **1.3.0**, `fountain_utility` **1.3.0**.
- Encoder/decoder examples use `Encoder::new_with_operator(&config, …)` (published `&CodeScheme` API).

### Packaging

- Crates.io tarball includes RFC 5053 **examples** (`test_raptor_10`, generator demos, `raptor_10_performance`).
- `raptor_10_performance` includes portable real-symbol benchmarks using `VecDataOperater`.
