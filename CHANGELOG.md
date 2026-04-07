# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [Unreleased]

- No unreleased changes yet.

## [0.3.0] - 2026-04-07

First workable public release of Rong.

- Unified multi-engine runtime surface across QuickJS, JavaScriptCore, and ArkJS.
- Worker-pool execution model for shared and pinned long-lived runtimes.
- Built-in runtime modules covering timers, HTTP, filesystem, buffer, URL, events,
  storage, streams, command execution, compression, Redis, SQLite, S3, and related
  host integration APIs.
- Runtime proxy APIs, including `JSProxy` support and cross-runtime proxy behavior.
- `rong_cli` for local runtime execution and REPL workflows.
