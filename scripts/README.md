# Publishing Scripts (Maintainer)

Recommended path: maintain the version and `CHANGELOG.md` manually, then use the
GitHub Actions `Release: Publish Packages` workflow to execute the release.

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

## publish_npm.sh

```
./scripts/publish_npm.sh
```

- Publishes all repo-maintained npm packages:
  - `@rongjs/rong` from `packages/rong_types`
  - `@rongjs/rong-skill` from `packages/skill`
- Runs only in GitHub Actions with npm Trusted Publishing through OIDC
- Skips the publish if the same npm version already exists
- First-time npm package creation must happen outside this repository automation
  before trusted publishing can be configured for that package

## GitHub publish flow (recommended)

1. Update the release version and `CHANGELOG.md`.
2. Land the release change on `master`.
3. GitHub → Actions → run workflow `Release: Publish Packages` from `master`
   with `package_scope=all`.

Notes:

- `Release: Publish Packages` reads the version from `Cargo.toml`.
- `Release: Publish Packages` requires a matching `CHANGELOG.md` entry for that version.
- `./scripts/bump_version.sh` syncs repo-maintained npm package versions with the workspace version.
- `Release: Publish Packages` publishes crates.io packages and all `@rongjs/*` npm packages, then creates the repository tag `vX.Y.Z` and the GitHub Release.
- `package_scope=rust` and `package_scope=npm` are package-only recovery paths;
  they skip the repository tag and GitHub Release.
- `Release: Publish Packages` requires `CARGO_REGISTRY_TOKEN` for Rust crates and
  npm trusted publisher configuration for npm packages.

## Local Rust recovery flow

Use this when GitHub Actions is unavailable or when you need to recover Rust
crate publishing manually. npm publishing is intentionally CI-only through
Trusted Publishing.

1. Run `./scripts/bump_version.sh <version>`.
2. Update `CHANGELOG.md` for the same version.
3. Review, commit, and push the release changes.
4. Run `./scripts/publish.sh` to publish crates.
5. Create tag `v<version>` and the GitHub Release manually.

## Troubleshooting

- Version exists on crates.io → bump patch version
- Publish fails mid-way → run `cargo publish -p <crate>`
