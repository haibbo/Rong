# Worker Execution Model

This document describes how Rong executes JavaScript work after the
`shared()` / `pinned()` builder split.

## Public API Surface

The public entrypoint is [`core/src/rong.rs`](../../core/src/rong.rs).

`Rong::builder()` is intentionally not directly buildable. Callers must choose
one execution model first:

- `Rong::builder().shared().build()`
- `Rong::builder().pinned::<K, S>().build()`

This keeps the placement model explicit in callsites. There is no implicit
"default shared" path via `builder().build()`.

`RongExecutor` sits one layer below these pool builders. It owns the
process-level Tokio runtime used by Rong host services and by any direct
host-side async work submitted with `RongExecutor::spawn(...)`.

## Module Boundaries

### `core/src/rong.rs`

This is the API facade layer.

It owns:

- public builder types
- public build-time validation
- public re-exports for shared worker types
- the rustdoc contract for `shared` vs `pinned`

It should not own worker-loop internals.

### `core/src/shared.rs`

This module implements the shared worker-pool model.

Shared mode means:

- tasks are dispatched to any available worker
- callers must not assume affinity to prior runs
- throughput is prioritized over sticky placement

This module owns:

- `Rong`
- `Worker`
- `TaskHandle`
- shared worker initialization
- shared worker-loop execution

### `core/src/pinned.rs`

This module implements the pinned worker-pool model.

Pinned mode means:

- the same key always maps to the same long-lived worker
- keyed state can be carried across invocations
- placement is deterministic for a given key

This module owns:

- `PinnedRong`
- `PinnedWorker`
- keyed state reuse
- pinned worker initialization
- pinned worker-loop execution

### `core/src/invoke.rs`

This module is not a worker-pool scheduler.

It owns the per-runtime JavaScript invoke queue:

- hard entry gating
- invoke ordering
- priority handling
- event coalescing

It is orthogonal to `shared` vs `pinned`.

### `core/src/worker_thread.rs`

This module owns the thread/runtime lifecycle glue shared by both pool models.

It centralizes:

- spawning JS worker threads
- installing current-thread Tokio runtimes
- worker-thread detection
- common join/shutdown behavior

This keeps `shared.rs` and `pinned.rs` focused on placement and state semantics
instead of duplicating thread bootstrap logic.

## Why `shared` and `pinned` Are Separate Concepts

`shared` and `pinned` describe placement semantics, not implementation details.

- `shared`: "run this on any available worker"
- `pinned`: "run this key on the same long-lived worker"

This is why the API uses:

- `shared().build()`
- `pinned::<K, S>().build()`

instead of a single `build()` with hidden defaults.

## Worker Thread Guardrails

Both shared and pinned worker threads are marked as "inside a Rong worker
thread". This matters for sync bridge APIs such as `call_blocking`, which must
reject re-entrant use from inside a worker thread.

The thread-local detection lives in `core/src/worker_thread.rs` so both pool
models enforce the same rule.
