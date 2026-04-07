# GitHub Workflows

## `ci.yml` (tests)

- **Trigger:** push + pull_request
- **Runs:** macOS matrix for `quickjs` and `jscore`
- **Steps:** `cargo fmt --check` → `cargo make check-engine` → `cargo make clippy-engine` → `cargo make test-engine`

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
