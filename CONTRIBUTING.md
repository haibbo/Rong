# Contributing

This project supports multiple JavaScript engines and a growing module set. Keep local checks and CI aligned by using the shared `cargo make` tasks.

## Development Setup

Requirements:

- Rust stable toolchain
- `cargo-make`
- On macOS, `llvm` is required for bindgen-based crates

Install helper tools:

```bash
cargo install cargo-make
brew install llvm
```

If `llvm` is installed by Homebrew, export `LIBCLANG_PATH` when needed:

```bash
export LIBCLANG_PATH="$(brew --prefix llvm)/lib"
```

## Local Verification

Fast local gate:

```bash
cargo make pre-commit
```

CI-equivalent verification for the default engine (`quickjs`):

```bash
cargo make ci-verify
```

Run the same verification for JavaScriptCore:

```bash
ENGINE=jscore cargo make ci-verify
```

Run the default CI engine set locally:

```bash
cargo make ci-verify-all
```

What these tasks do:

- `pre-commit`: `cargo fmt --check` + `cargo check` + `cargo clippy`
- `ci-verify`: `pre-commit` checks plus `bash test.sh -e <engine>`
- `ci-verify-all`: runs `ci-verify` sequentially for `quickjs` and `jscore`

The shared tasks exclude `rong_arkjs`, `rong_arkjs_sys`, and the device-only
`rong_test_device` crate from the default host CI gate. ArkJS/OHOS is validated
through separate Harmony-focused checks instead of the default host matrix. The
tasks also check `lib/bin/test` targets instead of `--all-targets` to avoid
example-only regressions from blocking normal work.

## Git Hooks

Hook layering:

- `pre-commit`: format-only, keeps each commit fast
- `pre-push`: runs `cargo make pre-commit`
- CI: runs the full verification flow

Install the local hooks:

```bash
git config --local core.hooksPath .githooks
```

Run them manually:

```bash
./.githooks/pre-commit
./.githooks/pre-push
```

## Test Matrix

- `quickjs`: default local engine; CI runs it on Windows and macOS
- `jscore`: secondary CI engine; CI runs it on macOS (system framework). It also
  builds and tests on other hosts via the source backend when a WebKit/JSCOnly
  artifact is available (see [`javascriptcore/sys/README.md`](javascriptcore/sys/README.md));
  `test.sh` skips `jscore` on non-Apple hosts that have no artifact configured.
- `arkjs`: verified separately through Harmony/OHOS checks; not part of the default host verification gate

For ad hoc engine tests, the lower-level runner remains available:

```bash
bash test.sh -e quickjs
bash test.sh -e jscore
```

For HarmonyOS device-side verification, use:

```bash
./testing/harmony/dev.sh test
```

For future self-hosted Harmony CI coverage, provision a local runner with
`OHOS_NDK_HOME` and the `harmony` runner label, then use the
`Harmony CI (Self-Hosted)` workflow.

## Release Flow

Preferred flow:

1. Prepare a normal PR with the version bump and matching `CHANGELOG.md` update.
2. Merge that PR into `master`.
3. Run `Release: Publish Packages` in GitHub Actions from `master`.

For local release details, see [`scripts/README.md`](scripts/README.md).
For the full maintainer checklist, see [`docs/releasing.md`](docs/releasing.md).

## Notes For Contributors

- Keep CI and local commands aligned. Prefer updating `Makefile.toml` instead of adding one-off commands to workflows.
- When adding or removing published crates, update `scripts/publish.sh`.
- If a check is engine-specific, be explicit about which engine it applies to.
