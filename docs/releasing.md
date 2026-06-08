# Releasing

This repository uses a maintainer-driven release flow:

- maintainers choose the version
- maintainers write `CHANGELOG.md`
- automation publishes crates and npm packages, creates the repository tag, and creates the GitHub Release

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

4. In GitHub Actions, run `Release: Publish Packages` from `master` with
   `package_scope=all`.

The publish workflow:

- reads the release version from `Cargo.toml`
- requires a matching `CHANGELOG.md` section
- publishes crates through `scripts/publish.sh` when `package_scope` is `all` or
  `rust`
- publishes all repo-maintained `@rongjs/*` npm packages through
  `scripts/publish_npm.sh` when `package_scope` is `all` or `npm`
- creates the repository tag as `vX.Y.Z` and the GitHub Release from the
  changelog entry only when `package_scope=all`

Requirements:

- `Release: Publish Packages` must run from `master`
- `CARGO_REGISTRY_TOKEN` must be configured in GitHub Actions
- `NPM_TOKEN` must be configured in GitHub Actions

`package_scope=rust` and `package_scope=npm` are recovery/partial-publish paths.
They publish only the selected package family and intentionally skip repository
tag and GitHub Release creation. Use `package_scope=all` for normal releases.

## Changelog Style

Write release notes for downstream users first, not as a commit log.

- Keep `## [Unreleased]` at the top.
- Add one section per release as `## [X.Y.Z] - YYYY-MM-DD`; the version must
  match `Cargo.toml` because the publish workflow extracts that exact section.
- Start formal releases with a short summary paragraph that explains the release
  outcome and audience.
- Use stable headings such as `Highlights`, `Added`, `Changed`, `Fixed`, and
  `Removed`.
- Prefer user-facing behavior, packaging changes, supported platforms, and
  migration-relevant details over internal commit or PR descriptions.
- Mention CI/release changes only when they affect contributors, package
  publication, artifact availability, or supported platforms.
- Keep generated GitHub Release notes self-contained; do not rely on surrounding
  `CHANGELOG.md` context.

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

5. Export the publish tokens:

   ```bash
   export CARGO_REGISTRY_TOKEN=...
   export NPM_TOKEN=...
   ```

6. Publish crates:

   ```bash
   ./scripts/publish.sh
   ```

   For non-interactive use:

   ```bash
   ./scripts/publish.sh --yes
   ```

7. Publish the npm packages:

   ```bash
   ./scripts/publish_npm.sh
   ```

8. Create the repository tag and GitHub Release manually:

   ```bash
   git tag -a v<version> -m "Rong v<version>"
   git push origin v<version>
   gh release create v<version> --title "v<version>" --notes-file <(bash ./scripts/extract_changelog_entry.sh <version>)
   ```

## Maintainer Notes

- `bump_version.sh` updates the workspace version, the root package version, internal workspace dependency versions, and repo-maintained npm package versions.
- `publish.sh` does not change versions or changelog content.
- `publish.sh` publishes crates in dependency order and waits for crates.io index propagation between packages.
- `publish_npm.sh` publishes all repo-maintained `@rongjs/*` npm packages and skips versions that already exist.
- When adding or removing published crates, update `scripts/publish.sh`.
- The GitHub release workflow's `package_scope=all` is the only path that creates
  the `vX.Y.Z` tag and GitHub Release; `rust` and `npm` are package-only paths.

## Short Version

- Normal release: open a normal PR with version + changelog changes, merge it, then run `Release: Publish Packages` with `package_scope=all`
- Fallback release: bump version, update changelog, run `publish.sh` and `publish_npm.sh`, then create `vX.Y.Z` tag and GitHub Release manually
