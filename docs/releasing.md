# Releasing

This repository uses a maintainer-driven release flow:

- maintainers choose the version
- maintainers write `CHANGELOG.md`
- automation publishes crates, creates the repository tag, and creates the GitHub Release

There is no generated release PR and no automatic version inference.

## Normal Flow

Use this for ordinary releases.

1. Prepare a normal release PR.
   The PR should include:
   - the version bump in `Cargo.toml`
   - any other versioned package metadata updates
   - the matching `CHANGELOG.md` entry

2. Run verification as needed:

   ```bash
   cargo make ci-verify-all
   ```

3. Merge the release PR into `master`.

4. In GitHub Actions, run `Release: Publish Crates` from `master`.

The publish workflow:

- reads the release version from `Cargo.toml`
- requires a matching `CHANGELOG.md` section
- publishes crates through `scripts/publish.sh`
- creates the repository tag as `vX.Y.Z`
- creates the GitHub Release from the changelog entry

Requirements:

- `Release: Publish Crates` must run from `master`
- `CARGO_REGISTRY_TOKEN` must be configured in GitHub Actions

## Local Fallback

Use this only when GitHub Actions is unavailable or when you are recovering from a partial release.

1. Run verification:

   ```bash
   cargo make pre-commit
   cargo make ci-verify-all
   ```

2. Update release metadata:

   ```bash
   ./scripts/bump_version.sh <version>
   ```

   Or create the version commit immediately:

   ```bash
   ./scripts/bump_version.sh <version> --commit
   ```

3. Update `CHANGELOG.md` for the same version.

4. Review, commit, and push the full release change if needed.

5. Export the crates.io token:

   ```bash
   export CARGO_REGISTRY_TOKEN=...
   ```

6. Publish crates:

   ```bash
   ./scripts/publish.sh
   ```

   For non-interactive use:

   ```bash
   ./scripts/publish.sh --yes
   ```

7. Create the repository tag and GitHub Release manually:

   ```bash
   git tag -a v<version> -m "Rong v<version>"
   git push origin v<version>
   gh release create v<version> --title "v<version>" --notes-file <(bash ./scripts/extract_changelog_entry.sh <version>)
   ```

## Maintainer Notes

- `bump_version.sh` updates the workspace version, the root package version, and internal workspace dependency versions.
- `publish.sh` does not change versions or changelog content.
- `publish.sh` publishes crates in dependency order and waits for crates.io index propagation between packages.
- When adding or removing published crates, update `scripts/publish.sh`.

## Short Version

- Normal release: open a normal PR with version + changelog changes, merge it, then run `Release: Publish Crates`
- Fallback release: bump version, update changelog, run `publish.sh`, then create `vX.Y.Z` tag and GitHub Release manually
