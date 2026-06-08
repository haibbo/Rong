## Summary

- 

## Validation

- [ ] `cargo make ci-verify`
- [ ] `ENGINE=jscore cargo make ci-verify` (if macOS / JavaScriptCore behavior changed)
- [ ] GitHub Actions `CI` passed for Windows/Linux/macOS `quickjs` and macOS `jscore`
- [ ] GitHub Actions `JSC Windows (source)` passed (if JavaScriptCore source backend changed)
- [ ] Windows/Linux `jscore-source-test` passed or intentionally skipped because no pinned prebuilt JSC artifact exists
- [ ] `build-jsc-artifacts.yml` was run and `javascriptcore/sys/webkit-artifacts.tsv` updated (if JSC source artifact version changed)
- [ ] `cargo make check-arkjs-ohos` (if ArkJS / OHOS code changed)
- [ ] `cargo make clippy-arkjs-ohos` (if ArkJS / OHOS code changed)
- [ ] `./testing/harmony/dev.sh test` on a device or local Harmony runner (if Harmony runtime behavior changed)
- [ ] Relied on GitHub Actions host CI for platform coverage
- [ ] Not run locally

If any relevant checks were skipped, explain why:

- 

## Release Impact

- [ ] No user-facing release note needed
- [ ] `CHANGELOG.md` updated
- [ ] Version bump included

## Notes

- 
