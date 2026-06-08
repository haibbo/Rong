# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [Unreleased]

## [0.3.2] - 2026-06-08

Rong 0.3.2 is the first formal public release of the project. It establishes the
maintainer-supported runtime surface, built-in module set, cross-platform
JavaScriptCore artifact path, and package publishing flow for downstream Rust and
JavaScript users.

### Highlights

- Unified Rust runtime API across QuickJS, JavaScriptCore, and ArkJS.
- Worker-pool execution model for shared runtimes and keyed long-lived runtimes.
- Built-in host modules for timers, HTTP/fetch, filesystem, buffer, URL, events,
  storage, streams, command execution, compression, Redis, SQLite, S3, workers,
  cron, and related runtime APIs.
- JavaScriptCore can use the system framework on Apple platforms or pinned
  source-built JSCOnly artifacts on macOS, Linux, and Windows.
- Release automation now publishes crates.io packages and all repo-maintained
  npm packages under the `@rongjs` scope.

### Added

- Added source-built JavaScriptCore support with pinned WebKit/JSCOnly artifact
  metadata in `javascriptcore/sys/webkit-artifacts.tsv`.
- Added the manual `Build JSC artifacts` workflow to produce macOS, Linux, and
  Windows JSC artifacts and validate normal CI consumption.
- Added the in-process `rong_cron` module.
- Added TypeScript package publishing for `@rongjs/rong`.
- Added installable agent skill packaging through `@rongjs/rong-skill`, with
  source documentation under `docs/skills` and generated API references from
  `docs/api`.
- Added release automation for publishing crates, npm packages, repository tags,
  and GitHub Releases from a single maintainer-run workflow.

### Changed

- Moved repo-maintained npm publishing to the `@rongjs` scope.
- Split CI into scoped jobs so docs-only changes avoid the Rust/JSC host matrix,
  while npm package changes still validate package generation.
- Expanded host CI coverage to QuickJS on Windows, Linux, and macOS; system
  JavaScriptCore on macOS; and source-built JavaScriptCore consumers on macOS,
  Linux, and Windows.
- Improved release scripts to publish packages in dependency order, tolerate
  crates.io index propagation delays, and skip already-published package
  versions during recovery.
- Removed the filesystem access guard from the filesystem module API.

### Fixed

- Fixed HTTP download redirect, timeout, abort, resume, and network-access guard
  behavior in `rong_rt`.
- Fixed `AbortSignal.timeout()` so Rust subscribers are notified correctly.
- Fixed ArkJS property value protection.
- Fixed array and object operation normalization across engines.
- Fixed package publish manifests and release verification for workspace crates,
  including `rong_rt` and `rong_cron`.
- Resolved clippy warnings across the workspace and relaxed timing-sensitive
  timer tests.

## [0.3.0] - 2026-04-07

Initial development release of Rong.

- Unified multi-engine runtime surface across QuickJS, JavaScriptCore, and ArkJS.
- Worker-pool execution model for shared and pinned long-lived runtimes.
- Built-in runtime modules covering timers, HTTP, filesystem, buffer, URL, events,
  storage, streams, command execution, compression, Redis, SQLite, S3, and related
  host integration APIs.
- Runtime proxy APIs, including `JSProxy` support and cross-runtime proxy behavior.
- `rong_cli` for local runtime execution and REPL workflows.
