# rong_quickjs

QuickJS backend for RongJS.

This crate integrates the vendored `rong_quickjs_sys` bindings with Rong's
engine-agnostic runtime traits and value model.

- Use `rong` if you want the public embedding API.
- Use `rong_quickjs` if you are wiring the QuickJS backend into lower-level
  Rong internals.
- Use `rong_quickjs_sys` only if you need the raw FFI layer.
