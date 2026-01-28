# Publishing Scripts (Maintainer)

Recommended path: use the **Release** workflow in GitHub Actions. Local scripts are here for manual use.

## bump_version.sh

```
./scripts/bump_version.sh <version> [--commit] [--tag] [--commit-and-tag]
```

- Updates `[workspace.package]` and syncs `[workspace.dependencies]`
- Default is file update only (no git ops)

## publish.sh

```
./scripts/publish.sh [--dry-run] [--no-verify] [--allow-dirty]
```

- Publishes all workspace crates in dependency order
- Requires `CARGO_REGISTRY_TOKEN` unless `--dry-run`
- Excludes: rong_arkjs* (WIP), rong_cli, rong_test, examples

## Local release (manual)

```
./scripts/bump_version.sh 0.1.2 --commit-and-tag
git push && git push --tags
export CARGO_REGISTRY_TOKEN=your_token
./scripts/publish.sh
```

## Troubleshooting

- Version exists on crates.io → bump patch version
- Tag exists → delete tag or choose a new version
- Publish fails mid-way → run `cargo publish -p <crate>`
