# GitHub Workflows

## `ci.yml` (tests)

- **Trigger:** push + pull_request
- **Runs:** `cargo fmt` once, then host verification on Windows `quickjs`, macOS `quickjs`, and macOS `jscore`
- **Steps:** `cargo fmt --check` → `cargo make ci-verify`

## `harmony-self-hosted.yml` (Harmony self-hosted)

- **Trigger:** manual (`workflow_dispatch`)
- **Runs:** ArkJS/OHOS `check`, `clippy`, and the Rust-side Harmony smoke-library build on a self-hosted runner with `OHOS_NDK_HOME`
- **Requirements:** runner labels `self-hosted` and `harmony`; intended for future local-runner coverage, not GitHub-hosted CI

## `release.yml` (manual publish)

- **Trigger:** manual (`workflow_dispatch`)
- **Runs:** validates the current workspace version and matching `CHANGELOG.md` entry, publishes crates, creates repo tag `vX.Y.Z`, and creates the GitHub Release from the changelog text
- **Requirements:** run from `master`; `CHANGELOG.md` must already contain the release entry

## Secrets

- `CARGO_REGISTRY_TOKEN` (required for publish)
- `GITHUB_TOKEN` (default Actions token; used to push the release tag and create the GitHub Release)

## Local testing

```bash
cargo make ci-verify
ENGINE=jscore cargo make ci-verify
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
