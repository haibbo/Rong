# GitHub Workflows

## `ci.yml` (tests)

- **Trigger:** push to `master`, pull_request, and manual `workflow_dispatch`
- **Concurrency:** PR and branch runs cancel older in-progress runs for the same PR/ref, so the Actions page shows the latest relevant CI instead of stale queued attempts.
- **Runs:** `cargo fmt` once, then separate host `check`, `clippy`, and `test` jobs:
  - `quickjs` on Windows, Linux, and macOS
  - `jscore` on macOS using the system `JavaScriptCore.framework`
  - `jscore-source-*` on macOS, Linux, and Windows, gated by pinned prebuilt artifact rows in `javascriptcore/sys/webkit-artifacts.tsv`
- **Source backend behavior:** `jscore-source-*` is the production-style prebuilt consumer path. It downloads and caches the pinned artifact through `rong_jscore_sys/build.rs`; if no row exists for a supported target, CI fails instead of silently skipping.
- **Steps:** `cargo fmt --check` plus `cargo make check-engine`, `cargo make clippy-engine`, and `cargo make test-engine`

## `build-jsc-artifacts.yml` (JSC source prebuilds)

- **Trigger:** manual (`workflow_dispatch`)
- **Runs:** builds WebKit/JSCOnly artifacts for supported macOS, Linux, and Windows targets and uploads tarballs to one GitHub Release.
- **Outputs:** per-target release assets plus TSV rows for `javascriptcore/sys/webkit-artifacts.tsv`.
- **Purpose:** produce the prebuilt source artifacts consumed by normal `jscore-source` builds. This keeps regular CI and local builds from compiling WebKit and keeps disk usage bounded to the downloaded artifact cache.
- **Update flow:** run the workflow with one release tag and either a WebKit tag/SHA or a branch that the workflow resolves to a fixed commit, review the emitted TSV rows, paste them into `javascriptcore/sys/webkit-artifacts.tsv`, then run `CI` to verify prebuilt consumption.

## `harmony-self-hosted.yml` (Harmony self-hosted)

- **Trigger:** manual (`workflow_dispatch`)
- **Runs:** ArkJS/OHOS `check`, `clippy`, and the Rust-side Harmony smoke-library build on a self-hosted runner with `OHOS_NDK_HOME`
- **Requirements:** runner labels `self-hosted` and `harmony`; intended for future local-runner coverage, not GitHub-hosted CI

## `release.yml` (manual publish)

- **Trigger:** manual (`workflow_dispatch`)
- **Runs:** validates the current workspace version and matching `CHANGELOG.md` entry, publishes crates and `@lingxia/rong`, creates repo tag `vX.Y.Z`, and creates the GitHub Release from the changelog text
- **Requirements:** run from `master`; `CHANGELOG.md` must already contain the release entry

## Secrets

- `CARGO_REGISTRY_TOKEN` (required for publish)
- `NPM_TOKEN` (required for npm publish)
- `GITHUB_TOKEN` (default Actions token; used to push the release tag and create the GitHub Release)

## Local testing

```bash
cargo make ci-verify
ENGINE=jscore cargo make ci-verify
RONG_JSC_SOURCE=1 ENGINE=jscore cargo make ci-verify
cargo make ci-verify-all
```

## Local hooks

```bash
git config --local core.hooksPath .githooks
./.githooks/pre-commit
./.githooks/pre-push
```

Local hooks are layered:

- `pre-commit`: `cargo fmt --all -- --check`
- `pre-push`: `cargo make pre-commit` (`fmt` + `check` + `clippy` with the default `quickjs` engine)

For local release steps, see `scripts/README.md`.
