# Release Checklist

This repository uses one release model:

- Maintainers decide the version and write `CHANGELOG.md`
- Automation publishes crates, creates the repo tag, and creates the GitHub Release

The preferred execution path is the GitHub Actions publish workflow. Local publishing
is the fallback when GitHub Actions is unavailable or when recovering from a partial
release.

## Preferred Flow: GitHub Actions publish

Checklist:

1. Land the intended changes on `master`.
2. Update the release metadata in the same change set:

   - bump the workspace version
   - add the matching `CHANGELOG.md` entry

3. Run local verification if needed:

   ```bash
   cargo make ci-verify-all
   ```

4. Merge the release commit or PR into `master`.
5. In GitHub Actions, run `Release: Publish` from `master`.

Notes:

- `Release: Publish` reads the release version from `Cargo.toml`.
- `Release: Publish` extracts the GitHub Release body from the matching `CHANGELOG.md` section.
- `Release: Publish` creates the repository tag as `vX.Y.Z`.
- `Release: Publish` requires the `CARGO_REGISTRY_TOKEN` GitHub secret.

## Local Fallback Flow

Use this when GitHub Actions is unavailable or when you need to recover manually.

Checklist:

1. Run verification:

   ```bash
   cargo make pre-commit
   cargo make ci-verify-all
   ```

2. Bump versions:

   ```bash
   ./scripts/bump_version.sh <version>
   ```

   Or create the version commit immediately:

   ```bash
   ./scripts/bump_version.sh <version> --commit
   ```

3. Update `CHANGELOG.md` with the matching release entry.
4. Review the combined release changes.
5. Commit and push the release changes if needed.
6. Export the crates.io token:

   ```bash
   export CARGO_REGISTRY_TOKEN=...
   ```

7. Publish:

   ```bash
   ./scripts/publish.sh
   ```

   For non-interactive use:

   ```bash
   ./scripts/publish.sh --yes
   ```

8. Create the repository tag and GitHub Release manually:

   ```bash
   git tag -a v<version> -m "Rong v<version>"
   git push origin v<version>
   gh release create v<version> --title "v<version>" --notes-file <(bash ./scripts/extract_changelog_entry.sh <version>)
   ```

Notes:

- `bump_version.sh` updates:
  - `[workspace.package]` version
  - the root `[package]` version
  - internal crate versions in `[workspace.dependencies]`
- `publish.sh` does not bump versions for you. Always bump first, then publish.
- `publish.sh` publishes crates in dependency order and waits for crates.io index propagation between packages.

## Quick Rule

- Normal release: merge version + changelog update -> run `Release: Publish`
- Fallback release: `bump_version.sh` -> update `CHANGELOG.md` -> `publish.sh` -> create `vX.Y.Z` tag + GitHub Release
