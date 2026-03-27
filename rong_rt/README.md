# rong_rt

Async runtime and host-side platform services for RongJS.

This crate provides the executor-facing runtime support used by Rong, including
HTTP client plumbing, async service integration, and transport-related helpers.

Most applications should depend on `rong` instead of using `rong_rt` directly.
