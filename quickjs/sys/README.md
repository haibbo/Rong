# rong_quickjs_sys

Low-level QuickJS-NG FFI bindings for RongJS.

This crate vendors the QuickJS-NG C sources used by Rong and exposes the raw
bindings plus a small compatibility shim layer required by the Rust FFI.

Most users should depend on `rong` or `rong_quickjs` instead of using this
crate directly.
