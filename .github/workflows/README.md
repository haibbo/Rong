# GitHub Workflows

## `ci.yml` (tests)

- **Trigger:** push + pull_request
- **Runs:** `test.sh` on macOS for `quickjs` and `jscore`

## `release.yml` (manual)

- **Trigger:** workflow_dispatch
- **Inputs:** `version` (required), `skip_tests`, `dry_run`, `no_verify`
- **Dry run:** preflight only (`cargo metadata` + `publish --dry-run`)
- **Full release:** bump version, commit+tag, push, publish, create GitHub Release

## Secrets

- `CARGO_REGISTRY_TOKEN` (required for publish)

## Local testing

```bash
bash test.sh -e quickjs
bash test.sh -e jscore
```

For local release steps, see `scripts/README.md`.
