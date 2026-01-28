# Publishing Scripts (Maintainer Guide)

> **Note:** This directory contains scripts for **maintainers** to publish releases. Regular contributors don't need these.

## Quick Start

### Option 1: GitHub Actions (Recommended)

1. Go to **Actions** → **Release** workflow
2. Click **Run workflow**
3. Enter version (e.g., `0.1.2`)
4. Run

Done! Fully automated: tests → bump → commit → tag → publish → GitHub Release.

### Option 2: Local Publishing

```bash
# Bump version (updates Cargo.toml, commits, tags)
./scripts/bump_version.sh 0.1.2 --commit-and-tag

# Push
git push && git push --tags

# Publish
export CARGO_REGISTRY_TOKEN=your_token
./scripts/publish.sh
```

## Scripts Reference

### `bump_version.sh`

```bash
./scripts/bump_version.sh <version> [--commit] [--tag] [--commit-and-tag]
```

**Default behavior:** Dry run (only updates Cargo.toml, no git operations)

- Updates `[workspace.package]` version
- Syncs all `[workspace.dependencies]` versions
- Use `--commit` to create git commit
- Use `--tag` to create git tag
- Use `--commit-and-tag` for both (recommended)

**Examples:**
```bash
# Dry run: just update files
./scripts/bump_version.sh 0.1.2

# Update and commit
./scripts/bump_version.sh 0.1.2 --commit

# Update, commit, and tag (recommended)
./scripts/bump_version.sh 0.1.2 --commit-and-tag
```

### `publish.sh`

```bash
./scripts/publish.sh [--dry-run] [--no-verify] [--allow-dirty]
```

- Publishes all crates to crates.io in dependency order
- Smart waiting: polls crates.io index until each package is available
- Default timeout: 60s, poll interval: 5s

## Publishing Order

```
Layer 1: rong_macro, rong_core
Layer 2: rong_quickjs_sys, rong_jscore_sys
Layer 3: rong_quickjs, rong_jscore
Layer 4: rong
Layer 5-7: All modules
Layer 8: rong_modules
```

**Not published:** `rong_arkjs*`, `rong_cli`, `rong_test`, `examples`

## Troubleshooting

**"Version already exists on crates.io"**
- You can't republish the same version
- Bump to next version (e.g., 0.1.2 → 0.1.3)

**"Tag already exists"**
- Delete tag: `git tag -d v0.1.2 && git push origin :refs/tags/v0.1.2`
- Or use a different version

**"Uncommitted changes"**
- Use `--allow-dirty` flag (not recommended)
- Or commit your changes first

**Publishing fails mid-way**
- Check which packages succeeded on crates.io
- Manually publish remaining packages: `cargo publish -p <crate-name>`

## Notes

- GitHub Actions requires `CARGO_REGISTRY_TOKEN` secret
- Get token from: https://crates.io/me
- Publishing can't be undone - only yanked
