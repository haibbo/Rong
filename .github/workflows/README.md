# GitHub Workflows

## `ci.yml` (tests)

- **Trigger:** push + pull_request
- **Runs:** macOS matrix for `quickjs` and `jscore`
- **Steps:** `cargo fmt --check` → `cargo make check-engine` → `cargo make clippy-engine` → `cargo make test-engine`

## `release-pr.yml` (release-plz)

- **Trigger:** manual (`workflow_dispatch`)
- **Runs:** release-plz to open/update a Release PR based on Conventional Commits
- **Labels:** applies `release` label to Release PRs (see `release-plz.toml`)
  - **Action:** `release-plz/action@v0.5`

## `release.yml` (release-plz)

- **Trigger:** manual (`workflow_dispatch`)
- **Runs:** publish workflow after Release PR merge (creates tag, publishes crates, creates GitHub Release)
  - **Action:** `release-plz/action@v0.5`

## Secrets

- `CARGO_REGISTRY_TOKEN` (required for publish)

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
