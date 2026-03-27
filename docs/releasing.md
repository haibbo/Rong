# Release Checklist

This repository supports two release paths:

- Preferred: GitHub Actions + `release-plz`
- Fallback: manual `bump_version.sh` + `publish.sh`

Do not mix the two flows for the same release.

## Recommended Flow: `release-plz`

Use this for normal releases.

Checklist:

1. Land the intended changes on `master`.
2. Run local verification if needed:

   ```bash
   cargo make ci-verify-all
   ```

3. In GitHub Actions, run `Release: Prepare PR`.
4. Review the generated Release PR.
5. Merge the Release PR into `master`.
6. In GitHub Actions, run `Release: Publish`.

Notes:

- `release-plz` updates versions in the Release PR.
- Do not run `./scripts/bump_version.sh` in this flow.
- Do not run `./scripts/publish.sh` in this flow unless you are intentionally recovering from a failed automation step.
- `Release: Publish` requires the `CARGO_REGISTRY_TOKEN` GitHub secret.

## Manual Flow: `bump_version.sh` + `publish.sh`

Use this only for emergencies or when intentionally bypassing `release-plz`.

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

   Or create the commit immediately:

   ```bash
   ./scripts/bump_version.sh <version> --commit
   ```

3. Review the version changes.
4. Export the crates.io token:

   ```bash
   export CARGO_REGISTRY_TOKEN=...
   ```

5. Publish:

   ```bash
   ./scripts/publish.sh
   ```

   For non-interactive use:

   ```bash
   ./scripts/publish.sh --yes
   ```

6. Create any required Git tags or GitHub Releases manually if you are not returning to the `release-plz` flow afterward.

Notes:

- `bump_version.sh` updates:
  - `[workspace.package]` version
  - the root `[package]` version
  - internal crate versions in `[workspace.dependencies]`
- `publish.sh` does not bump versions for you. Always bump first, then publish.
- `publish.sh` publishes crates in dependency order and waits for crates.io index propagation between packages.

## Which One To Use

Use `release-plz` when:

- you are doing a normal project release
- GitHub Actions is available
- you want versioning and publishing to stay aligned with the existing automation

Use the manual flow when:

- you are recovering from a broken automated release
- you explicitly need to bypass `release-plz`
- you are doing a one-off maintainer release and accept the extra manual bookkeeping

## Quick Rule

- Normal release: `Release: Prepare PR` -> merge -> `Release: Publish`
- Manual release: `bump_version.sh` -> review/commit -> `publish.sh`
