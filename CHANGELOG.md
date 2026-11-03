# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.6](https://github.com/bodhispace-xyz/pbs-exporter-rs/compare/pbs-exporter-rs-v0.4.5...pbs-exporter-rs-v0.4.6) (2026-08-24)


### Performance Improvements

* **ci:** speed up CI with pre-built cargo-audit binary and parallel release build ([6a85f12](https://github.com/bodhispace-xyz/pbs-exporter-rs/commit/6a85f12957b5b025df1594e8ea035c58ff12bb5b))
* **ci:** speed up CI with pre-built cargo-audit binary and parallel release build ([51ca1de](https://github.com/bodhispace-xyz/pbs-exporter-rs/commit/51ca1de36bfd8a2a9287f9f40d5fb2cb1c6db4c2))

## [0.4.5](https://github.com/bodhispace-xyz/pbs-exporter-rs/compare/pbs-exporter-rs-v0.4.4...pbs-exporter-rs-v0.4.5) (2026-08-22)


### Features

* optimise memory and add snapshot verification metrics ([b59a979](https://github.com/bodhispace-xyz/pbs-exporter-rs/commit/b59a979bb80a211f11316a5104044aa894a335db))
* optimize memory and add snapshot verification metrics ([d5bf63b](https://github.com/bodhispace-xyz/pbs-exporter-rs/commit/d5bf63b35ef8caec44ad79183cff06a2abf74997))


### Bug Fixes

* **metrics:** track maximum endtime for task_last_run_timestamp ([#27](https://github.com/bodhispace-xyz/pbs-exporter-rs/issues/27)) ([0a35264](https://github.com/bodhispace-xyz/pbs-exporter-rs/commit/0a35264b486be82ae7a97acabd9bfc4344a6b803))

## [Unreleased]

### Added

- Initial release of PBS Exporter
- Comprehensive Prometheus metrics for PBS 4.x
  - Host system metrics (CPU, memory, disk, load)
  - Datastore capacity metrics
  - Backup snapshot metrics with verification status
  - Individual snapshot metrics with history limiting
  - Task metrics (duration, status, running count)
  - Garbage collection metrics
  - Tape drive metrics
- Self-monitoring metrics:
  - `pbs_exporter_scrape_duration_seconds` - Scrape performance tracking
  - `pbs_exporter_memory_usage_bytes` - Memory usage monitoring
- PBS API client with authentication support
- HTTP server with `/metrics`, `/health`, and `/` endpoints
- Configuration via TOML files and environment variables
- CLI with `--config` flag
- Structured logging with tracing
- Docker support with multi-stage builds
- Comprehensive documentation with examples for all public APIs
- Snapshot history limiting for cardinality control (`snapshot_history_limit` config)

### Changed

- **Major refactoring**: Split monolithic `metrics.rs` (1,086 lines) into 4 focused modules:
  - `mod.rs` - Public API (48 lines)
  - `registry.rs` - Metric definitions with builder pattern (346 lines)
  - `collectors.rs` - Collection orchestration (225 lines)
  - `updates.rs` - Metric update functions (389 lines)
- Implemented MetricBuilder pattern reducing metric registration code by 70%
- Optimized memory usage with Entry API pattern for HashMap operations
- Reduced allocation overhead in metric collection (~58-100 KB savings per cycle)

### Fixed

- Memory efficiency improvements:
  - Eliminated duplicate HashMap key cloning using Entry API
  - Optimized lookup operations to clone keys only once
  - Removed buffer cloning in encode() using `std::mem::take`
- Enhanced error handling with proper graceful degradation

### Testing

- Increased test coverage from 50% to 80%
- Added 33 new tests with Given-When-Then structure:
  - Snapshot metrics tests (comment selection, verification, protection)
  - Task metrics tests (duration, status, grouping)
  - GC status tests (OK/ERROR conversion, optional fields)
  - Cardinality limiting tests (history limits 0, 1, 5)
  - Tape drive metrics tests
- Total: 49 passing tests
- All tests use Given-When-Then documentation format
