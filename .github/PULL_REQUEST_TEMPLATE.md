## Summary

- 

## Validation

- [ ] `cargo make ci-verify`
- [ ] `ENGINE=jscore cargo make ci-verify` (if macOS / JavaScriptCore behavior changed)
- [ ] `cargo make check-arkjs-ohos` (if ArkJS / OHOS code changed)
- [ ] `cargo make clippy-arkjs-ohos` (if ArkJS / OHOS code changed)
- [ ] `./testing/harmony/dev.sh test` on a device or local Harmony runner (if Harmony runtime behavior changed)
- [ ] Relied on GitHub Actions host CI for Windows `quickjs` / macOS `jscore`
- [ ] Not run locally

If any relevant checks were skipped, explain why:

- 

## Release Impact

- [ ] No user-facing release note needed
- [ ] `CHANGELOG.md` updated
- [ ] Version bump included

## Notes

- 
