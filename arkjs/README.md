# rong_arkjs

Rust bindings for OpenHarmony ArkJS JavaScript engine.

## Overview

This crate provides Rust bindings for the ArkJS JavaScript engine used in OpenHarmony OS. ArkJS is the JavaScript runtime that powers HarmonyOS applications.

## Platform Support

`rong_arkjs` is only supported on HarmonyOS/OpenHarmony targets such as:

- `aarch64-unknown-linux-ohos`

You must install the HarmonyOS/OpenHarmony native toolchain and set:

```bash
export OHOS_NDK_HOME=/path/to/ohos-sdk
```

Then build with:

```bash
cargo build --target aarch64-unknown-linux-ohos
```
