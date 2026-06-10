# Releasing

This repository uses a maintainer-driven release flow:

- maintainers choose which packages are released
- maintainers choose each package version
- maintainers write `CHANGELOG.md`
- automation publishes selected crates and npm packages

There is no generated release PR, no automatic version inference, and no CI-created
repo-level product tag.

## Version Model

Rust crates are versioned independently. Do not bump every workspace crate just
because one implementation crate changed.

Think from the downstream dependency entry point:

- Application authors commonly depend on entry crates such as `rong`,
  `rong_http`, `rong_timer`, or `rong_modules`.
- Engine crates such as `rong_jscore`, `rong_jscore_sys`, `rong_quickjs`, and
  `rong_quickjs_sys` are implementation crates and can receive patch releases
  independently.
- A dependent crate only needs a new version when its own API/behavior changed
  or when it must raise the minimum compatible version of one of its dependencies.

For example, a `rong_jscore_sys` linking fix can usually release only
`rong_jscore_sys` and `rong_jscore`. A `rong_timer` behavior fix can usually
release only `rong_timer`. `rong` or `rong_modules` should be released only when
their own behavior or dependency lower bounds need to change.

## Normal Flow

Use this for ordinary package releases.

1. Decide the release set:

   ```bash
   # Examples
   ./scripts/publish.sh --crate rong_timer --dry-run
   ./scripts/publish.sh --crate rong_jscore_sys --crate rong_jscore --dry-run
   ./scripts/publish.sh --group engines --dry-run
   ```

   Use `--changed-since <ref>` as a planning aid, then review the generated plan.

2. Prepare a normal release PR. The PR should include:
   - version bumps for the selected Rust crates and/or npm packages
   - matching `CHANGELOG.md` entries
   - any dependency lower-bound bumps that are intentionally required

   Examples:

   ```bash
   ./scripts/bump_version.sh 0.4.1 --crate rong_timer
   ./scripts/bump_version.sh 0.4.1 --crate rong_jscore_sys --crate rong_jscore
   ./scripts/bump_version.sh 0.4.1 --group npm
   ```

3. Run verification as needed:

   ```bash
   cargo make ci-verify-all
   ```

4. Merge the release PR into `master`.

5. In GitHub Actions, run `Publish Packages` from `master`:
   - `package_scope=rust`, `npm`, or `all`
   - `rust_selection` such as `--crate rong_timer`, `--group engines`, or
     `--changed-since v0.4.0`
   - `create_tags=true` when package-level git tags should be pushed

The publish workflow:

- publishes selected crates through `scripts/publish.sh`
- publishes repo-maintained `@rongjs/*` npm packages through
  `scripts/publish_npm.sh`
- optionally creates package-level tags such as `rong_timer-v0.4.1`,
  `rong_jscore-v0.4.1`, or `npm-rongjs-rong-v0.4.1`
- does not create repo-level product tags such as `v0.4.1`
- does not create GitHub Releases

Requirements:

- `Publish Packages` must run from `master`
- `CARGO_REGISTRY_TOKEN` must be configured in GitHub Actions for Rust publishes
- npm trusted publishing must be configured for each repo-maintained npm package

## Product Tags

Repo-level product tags such as `v0.4.0` are explicit maintainer decisions. They
mark a product-level release point, not every package publish.

Create them manually when the released package set should be treated as a
cohesive product release:

```bash
git tag -a v<version> -m "Rong v<version>"
git push origin v<version>
```

Create a GitHub Release only when a product-level release note is useful.

## npm Trusted Publishing

The publish workflow publishes npm packages with npm Trusted Publishing (GitHub
Actions OIDC), not token-based npm credentials.

Before the workflow can publish a package, npm must know that package and its
trusted publisher:

- `@rongjs/rong` publishes from [`packages/rong_types`](../packages/rong_types)
- `@rongjs/rong-skill` publishes from [`packages/skill`](../packages/skill)
- GitHub repository: `LingXia-Dev/Rong`
- Workflow file: `.github/workflows/release.yml`

npm trusted publisher configuration is package-level, so the package must exist
before the trusted publisher can be attached. For the first `@rongjs/*` publish,
create the package outside this repository automation, then add the trusted
publisher in npm package settings and use `Publish Packages` for subsequent
releases.

The workflow has `id-token: write` permission and uses npm CLI 11.5.1+ so npm can
exchange the GitHub OIDC token during `npm publish`.

## Changelog Style

Write release notes for downstream users first, not as a commit log.

- Keep `## [Unreleased]` at the top.
- Use package/family headings when package versions differ, for example
  `JavaScriptCore`, `Timer`, `npm`, or `Release tooling`.
- Start formal product releases with a short summary paragraph that explains the
  release outcome and audience.
- Prefer user-facing behavior, packaging changes, supported platforms, and
  migration-relevant details over internal commit or PR descriptions.
- Mention CI/release changes only when they affect contributors, package
  publication, artifact availability, or supported platforms.

## Manual Rust Recovery

Use this only when GitHub Actions is unavailable or when you are recovering the
Rust crate publish path manually. npm publishing is intentionally CI-only through
Trusted Publishing.

1. Run verification:

   ```bash
   cargo make pre-commit
   cargo make ci-verify-all
   ```

2. Bump the selected package versions:

   ```bash
   ./scripts/bump_version.sh 0.4.1 --crate rong_timer
   ```

3. Update `CHANGELOG.md`.

4. Review, commit, and push the release change if needed.

5. Export the crates.io publish token:

   ```bash
   export CARGO_REGISTRY_TOKEN=...
   ```

6. Publish matching crates:

   ```bash
   ./scripts/publish.sh --crate rong_timer --tag
   ```

## Maintainer Notes

- `bump_version.sh` changes selected package versions only.
- `publish.sh` does not change versions or changelog content.
- `publish.sh` publishes crates in dependency order and waits for crates.io index
  propagation between packages.
- `publish_npm.sh` publishes all repo-maintained `@rongjs/*` npm packages and
  skips versions that already exist. It runs only in GitHub Actions with trusted
  publishing.
- When adding or removing published crates, update `scripts/publish.sh`.
- When adding or removing repo-maintained npm packages, update
  `scripts/publish_npm.sh`.
