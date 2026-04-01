# Rong (融) - JavaScript Runtime for Rust

[![Rust](https://img.shields.io/badge/rust-1.90+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-APACHE)

**Rong** is a JavaScript runtime for Rust with a unified API over multiple JS
engines. It is designed for embedding, Rust-driven JS APIs, and long-lived
worker runtimes.

## 🌟 Why "Rong" (融)?

The name "Rong" (融) embodies the core philosophy of this project:

- **Fusion**: Seamlessly merges JavaScript engines with Rust native code
- **Harmony**: Creates harmonious integration between different runtime environments
- **Unity**: Unifies multiple JavaScript engines under a single, elegant API
- **Flow**: Enables smooth data flow between JavaScript and Rust worlds

In Chinese culture, "融" represents natural harmony and coexistence - perfectly capturing how Rong brings together diverse technologies into a unified whole.

## Features

### Multi-Engine Support
- **QuickJS** - Lightweight and fast
- **JavaScriptCore** - WebKit's production-ready engine
- **ArkJS** - HarmonyOS JavaScript engine

### Developer Experience
- **Type-safe bindings** - Rust's type system helps keep host bindings safe
- **Async/await support** - Promise and async iterator integration
- **Worker pools** - Shared and pinned execution models
- **Rich module ecosystem** - Built-in modules for common tasks

### Architecture
- **Unified API** - Same code works across all engines
- **Memory efficient** - Careful resource management
- **Thread-safe** - Safe concurrent access patterns
- **Extensible** - Easy to add custom modules and bindings

```
┌─────────────────────────────────────────────────────────────┐
│                        Rong Core                            │
├─────────────────────────────────────────────────────────────┤
│ Unified API │ Type System │ Memory Management │ Async/Await │
├─────────────────────────────────────────────────────────────┤
│    QuickJS   │      JavaScriptCore     │      ArkJS         │
├─────────────────────────────────────────────────────────────┤
│              Built-in Modules & Extensions                  │
│   Timer │ HTTP │ FS │ Console │ S3 │ SQLite │ Redis │ ...   │
└─────────────────────────────────────────────────────────────┘
```


## 🚀 Quick Start

```rust
use rong::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a JavaScript runtime and context
    let rt = RongJS::runtime();
    let ctx = rt.context();

    // Execute JavaScript code
    let result: i32 = ctx.eval(Source::from_bytes(b"2 + 3"))?;
    println!("Result: {}", result); // Output: Result: 5

    Ok(())
}
```

This is the lowest-level embedding path: create a runtime, create a context,
and evaluate JavaScript directly.

## Common Usage: Worker Pools

Most applications move one step up from raw `RongJS::runtime()` and use a worker
pool:

- `shared()` for stateless work that can run on any available worker
- `pinned::<K, S>()` for keyed work that must stay on the same long-lived worker

```rust
use rong::{Rong, RongJS, Source};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rong = Rong::<RongJS>::builder().shared().workers(2).build()?;
    let value: i32 = rong.call(|runtime, _receiver| async move {
        let ctx = runtime.context();
        ctx.eval(Source::from_bytes(b"21 * 2"))
    }).await?;

    println!("JS result: {value}");
    Ok(())
}
```

You must choose an execution model explicitly. There is no implicit
`builder().build()` default.

## Advanced Runtime Control

Most users do not need to construct `RongExecutor` directly. Rong host services
and worker pools use the process-global executor, and a default one is created
on first use.

Reach for `RongExecutor::builder()` only when you need to:

- customize host executor thread count or thread names
- install a custom global executor up front
- submit host-side async work directly with `RongExecutor::spawn(...)`

## Examples and Guides

- [`examples/src/worker.rs`](examples/src/worker.rs) for a runnable shared worker example
- [`examples/src/point.rs`](examples/src/point.rs) for class bindings
- [`examples/src/executor.rs`](examples/src/executor.rs) for custom `RongExecutor` setup
- [Worker Execution Model](docs/internals/worker_execution_model.md) for `shared` vs `pinned`
- [Module Development Guide](docs/internals/module_development.md) for writing
  Rust-driven JS APIs, classes, and modules

### Engine Selection

QuickJS is the default engine:

```bash
cargo run -p rong_cli
```

Switch to JavaScriptCore explicitly:

```bash
cargo run -p rong_cli --no-default-features --features jscore,tls-aws-lc
```

Build for ArkJS explicitly on HarmonyOS/OpenHarmony targets:

```bash
cargo build --no-default-features --features arkjs --target aarch64-unknown-linux-ohos
```

`quickjs`, `jscore`, and `arkjs` are mutually exclusive. If multiple engines are enabled, build fails fast.
For TLS backend selection, use `tls-aws-lc` (default) or `tls-ring`.

## 📦 Built-in Modules

Rong comes with a rich set of built-in modules:

- **rong_timer** - setTimeout, setInterval, async timers
- **rong_http** - HTTP client/server, fetch API
- **rong_fs** - File system operations
- **rong_console** - Console logging and debugging
- **rong_url** - URL parsing and manipulation
- **rong_buffer** - Binary data handling
- **rong_event** - Event emitter and handling
- **rong_abort** - AbortController and signals
- **rong_encoding** - Text encoding/decoding
- **rong_assert** - Assertion utilities
- **rong_storage** - Storage APIs
- **rong_stream** - Stream APIs
- **rong_command** - Command execution APIs for subprocesses and shell commands
- **rong_cron** - Cron parsing and macOS scheduled job registration
- **rong_exception** - Exception handling
- **rong_sqlite** - SQLite APIs
- **rong_s3** - S3-compatible object storage APIs

## 📚 Documentation

- **[Contributing Guide](CONTRIBUTING.md)** - Local setup, verification, hooks, and release workflow
- **[Module Development Guide](docs/internals/module_development.md)** - Learn how to create Rust-driven JavaScript APIs and classes
- **[Worker Execution Model](docs/internals/worker_execution_model.md)** - Understand `shared` vs `pinned` workers and internal runtime boundaries
- **[Value System Guide](docs/internals/value_system.md)** - Understand type conversion between Rust and JavaScript
- **[Error Handling Guide](docs/internals/error_handling.md)** - Best practices for error handling
- **[Testing Guide](docs/internals/testing.md)** - How to run and write tests

## 📄 License

This project is licensed under either the MIT License or the Apache License 2.0, at your option.
See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for details.

## 🙏 Acknowledgments

- QuickJS team for the lightweight JavaScript engine
- WebKit team for JavaScriptCore
- HarmonyOS team for ArkJS
- Rust community for excellent async ecosystem

---

**Rong (融)** - *Fusing JavaScript engines with Rust, creating harmony in diversity.*
