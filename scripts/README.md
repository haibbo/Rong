# Publishing Scripts (Maintainer)

Recommended path: maintain the version and `CHANGELOG.md` manually, then use the
GitHub Actions `Release: Publish` workflow to execute the release.

Release flow summary: see [`docs/releasing.md`](../docs/releasing.md).

## Local verification

```bash
cargo make pre-commit
cargo make ci-verify
ENGINE=jscore cargo make ci-verify
cargo make ci-verify-all
```

- `cargo make pre-commit`: fast local gate (`cargo fmt --check` + `cargo check` + `cargo clippy`)
- `cargo make ci-verify`: CI-equivalent gate for one engine, including `bash test.sh -e <engine>`
- `cargo make ci-verify-all`: runs `ci-verify` sequentially for all default CI engines

Optional local hook setup:

```bash
git config --local core.hooksPath .githooks
./.githooks/pre-commit
./.githooks/pre-push
```

- `pre-commit` hook only runs `cargo fmt --all -- --check`
- `pre-push` hook runs `cargo make pre-commit`

## bump_version.sh

```
./scripts/bump_version.sh <version> [--commit]
```

- Updates `[workspace.package]` and syncs `[workspace.dependencies]`
- Default is file update only (no git ops)
- Does not create tags or GitHub releases

## publish.sh

```
./scripts/publish.sh [--no-verify] [--allow-dirty] [--yes]
```

- Publishes all publishable workspace crates in dependency order, including `rong_cli`
- Requires `CARGO_REGISTRY_TOKEN`
- Smart waiting: polls crates.io until each package is indexed
- `--yes` skips the confirmation prompt (useful for CI)

## GitHub publish flow (recommended)

1. Update the release version and `CHANGELOG.md`.
2. Land the release change on `master`.
3. GitHub → Actions → run workflow `Release: Publish` from `master`.

Notes:

- `Release: Publish` reads the version from `Cargo.toml`.
- `Release: Publish` requires a matching `CHANGELOG.md` entry for that version.
- `Release: Publish` creates the repository tag `vX.Y.Z` and the GitHub Release.
- `Release: Publish` requires `CARGO_REGISTRY_TOKEN` secret to publish to crates.io.

## Local fallback flow

Use this when GitHub Actions is unavailable or when you need to recover manually:

1. Run `./scripts/bump_version.sh <version>`.
2. Update `CHANGELOG.md` for the same version.
3. Review, commit, and push the release changes.
4. Run `./scripts/publish.sh` to publish crates.
5. Create tag `v<version>` and the GitHub Release manually.

## Troubleshooting

- Version exists on crates.io → bump patch version
- Publish fails mid-way → run `cargo publish -p <crate>`
