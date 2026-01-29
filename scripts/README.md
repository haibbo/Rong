# Publishing Scripts (Maintainer)

Recommended path: use the **GitHub Actions** release-plz workflows. Local scripts are here for manual use / emergencies.

## bump_version.sh

```
./scripts/bump_version.sh <version> [--commit]
```

- Updates `[workspace.package]` and syncs `[workspace.dependencies]`
- Default is file update only (no git ops)
- Does not create tags (release-plz uses per-package tags)

## publish.sh

```
./scripts/publish.sh [--no-verify] [--allow-dirty] [--yes]
```

- Publishes all workspace crates in dependency order
- Requires `CARGO_REGISTRY_TOKEN`
- Excludes: rong_arkjs* (WIP), rong_cli, rong_test, examples
- Smart waiting: polls crates.io until each package is indexed
- `--yes` skips the confirmation prompt (useful for CI)

## GitHub release flow (recommended, manual)

1. Land changes on `master` (prefer Conventional Commits: `fix: ...`, `feat: ...`, `feat!: ...`).
2. GitHub → Actions → run workflow `Release: Prepare PR` (select branch `master`).
3. Review and merge the generated “Release PR” (this PR contains the version bumps + changelog updates).
4. GitHub → Actions → run workflow `Release: Publish` (select branch `master`).

Notes:
- The “version bump” is done by release-plz inside the Release PR; you generally do **not** run `bump_version.sh` for the GitHub-based flow.
- `Release: Publish` requires `CARGO_REGISTRY_TOKEN` secret to publish to crates.io.
  - The GitHub workflows use `release-plz/action@v0.5` (latest v0.5.x).

## Local manual flow (not recommended)

Use this only if you intentionally want to bypass release-plz automation:

1. Run `./scripts/bump_version.sh <version>` and commit the changes.
2. Run `./scripts/publish.sh` to publish crates.
3. Create Git tags / GitHub Releases manually as needed.

## Troubleshooting

- Version exists on crates.io → bump patch version
- Publish fails mid-way → run `cargo publish -p <crate>`
