# Publishing Scripts (Maintainer)

Recommended path: merge the **Release PR** created by release-plz. Local scripts are here for manual use.

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

## Release flow (recommended)

```bash
# 1. Land Conventional Commits on master
# 2. release-plz opens a Release PR
# 3. Merge the Release PR
# 4. GitHub Actions publishes automatically
```

## Troubleshooting

- Version exists on crates.io → bump patch version
- Publish fails mid-way → run `cargo publish -p <crate>`
