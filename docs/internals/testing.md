# Testing

Rong is a multi-engine project. Most tests need an engine feature enabled.
By default, `rong` uses QuickJS. To switch to JavaScriptCore, always use
`--no-default-features --features jscore` to avoid enabling both engines.

## Cargo tests

### Running all tests

```bash
# QuickJS (default on `rong`)
cargo test

# JavaScriptCore
cargo test --no-default-features --features jscore
```

### Testing a specific module

To test a single module, use the `-p` (package) flag:

```bash
# Test rong_http module with QuickJS
cargo test -p rong_http --features quickjs

# Test rong_timer module with JavaScriptCore
cargo test -p rong_timer --features jscore

# Test rong_fs module with QuickJS
cargo test -p rong_fs --features quickjs
```

**Available modules**:
- `rong_http` - HTTP client (fetch)
- `rong_timer` - setTimeout/setInterval
- `rong_fs` - File system operations
- `rong_console` - Console logging
- `rong_buffer` - Binary data handling
- `rong_encoding` - Text encoding/decoding
- `rong_event` - Event emitter
- `rong_abort` - AbortController
- `rong_url` - URL parsing
- `rong_path` - Path manipulation
- `rong_stream` - Stream APIs
- `rong_process` - Process information
- `rong_child_process` - Child process management
- `rong_storage` - Storage APIs
- `rong_assert` - Assertion utilities
- `rong_exception` - Exception handling
- `rong_navigator` - Navigator APIs
- `rong_redis` - Redis client
- `rong_sqlite` - SQLite database
- `rong_s3` - S3 object storage

### Testing multiple modules

```bash
# Test all workspace packages
cargo test --workspace

# Test specific modules
cargo test -p rong_http -p rong_timer --features quickjs
```

### Running specific test cases

```bash
# Run a specific test function in a module
cargo test -p rong_http test_fetch --features quickjs

# Run all tests matching a pattern
cargo test -p rong_timer timeout --features quickjs

# Show test output
cargo test -p rong_http --features quickjs -- --nocapture
```

## Module test runner

The repository also provides a small test runner script to execute a single module test suite
against a specific engine:

```bash
# Test rong_http with QuickJS
./test.sh -e quickjs -t rong_http

# Test rong_http with JavaScriptCore
./test.sh -e jscore -t rong_http

# Test rong_timer with QuickJS
./test.sh -e quickjs -t rong_timer
```

This script is useful for:
- Quick module testing during development
- CI/CD integration
- Testing across different engines
