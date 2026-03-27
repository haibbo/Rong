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

The shared tasks intentionally exclude `rong_arkjs` and `rong_arkjs_sys` for now, and they check `lib/bin/test` targets instead of `--all-targets`. That keeps the main CI gate focused on supported engines and avoids example-only regressions from blocking normal work.

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

- `quickjs`: default local and CI engine
- `jscore`: secondary CI engine
- `arkjs`: work in progress, not part of the default verification gate

For ad hoc engine tests, the lower-level runner remains available:

```bash
bash test.sh -e quickjs
bash test.sh -e jscore
```

## Release Flow

Preferred flow:

1. Land changes on `master`.
2. Run `Release: Prepare PR` in GitHub Actions.
3. Review and merge the generated release PR.
4. Run `Release: Publish` in GitHub Actions.

For local release details, see [`scripts/README.md`](scripts/README.md).
For the release checklist and the difference between `release-plz` and manual publishing, see [`docs/releasing.md`](docs/releasing.md).

## Notes For Contributors

- Keep CI and local commands aligned. Prefer updating `Makefile.toml` instead of adding one-off commands to workflows.
- When adding or removing published crates, update `scripts/publish.sh`.
- If a check is engine-specific, be explicit about which engine it applies to.
