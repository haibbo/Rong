# GitHub Workflows

## `ci.yml` (tests)

- **Trigger:** push + pull_request
- **Runs:** `test.sh` on macOS for `quickjs` and `jscore`

## `release-pr.yml` (release-plz)

- **Trigger:** push to `master`
- **Runs:** release-plz to open/update a Release PR based on Conventional Commits
- **Labels:** applies `release` label to Release PRs (see `release-plz.toml`)

## `release.yml` (release-plz)

- **Trigger:** CI workflow success on `master` (push only)
- **Runs:** publish workflow after Release PR merge (creates tag, publishes crates, creates GitHub Release)

## Secrets

- `CARGO_REGISTRY_TOKEN` (required for publish)

## Local testing

```bash
bash test.sh -e quickjs
bash test.sh -e jscore
```

For local release steps, see `scripts/README.md`.
