# Error handling at the Rust<->JS boundary

Two concepts:

- **`RongError`** - a host-generated Error-like object with `name` / `message` / `code` / `data`.
- **thrown payload** - any preserved JS value (`throw 1`, a promise rejection reason, an abort reason).

## Three core rules

1. **Rust `Err` = JS throw/reject.** No exceptions:
   - `Ok(v)` -> normal return / `Promise.resolve(v)`
   - `Err(e)` -> `throw` / `Promise.reject(payload)`
   `JSResult<T>` is the bridge type for sync and async host functions alike.
2. **Detect exceptions with `value.is_exception()`** - *not* `is_error()`. In JS,
   `return new Error("x")` is a normal value; only `throw` enters the exception channel.
3. **Host failures use `HostError`; JS-thrown values use `RongJSError::from_thrown_value`.**

| Source            | Representation                       |
| :---              | :---                                 |
| Rust-side failure | `HostError::new(code, msg)`          |
| JS-side throw     | `RongJSError::from_thrown_value(v)`  |

## HostError API

```rust
use rong::error::{E_INVALID_ARG, E_IO, E_TYPE, HostError, JSResult};

HostError::new(code, message)            // basic
    .with_name("TypeError")              // JS Error name (TypeError, RangeError, AbortError, ...)
    .with_data(rong::err_data!({ key: "value" }))  // structured details (-> e.data in JS)
    .into()                              // HostError -> RongJSError

HostError::invalid_arg_count(expected, got)  // convenience
HostError::aborted(None)                     // default AbortError
```

Example with structured data:

```rust
fn read_file(path: &str) -> JSResult<Vec<u8>> {
    std::fs::read(path).map_err(|e| {
        HostError::new(E_IO, e.to_string())
            .with_data(rong::err_data!({
                path: path,
                os_error: (e.raw_os_error()),  // Option<i32> -> number | null
            }))
            .into()
    })
}
```

What JS sees:

```js
try { await readFile("/etc/shadow"); }
catch (e) {
  e.name;    // "Error"
  e.message; // "..."
  e.code;    // "E_IO" (stable, for programmatic handling)
  e.data;    // { path: "..." }
}
```

## Error codes (`rong::error::E_*`)

| Code                    | Meaning                   | Typical `.with_name()`          |
| :---                    | :---                      | :---                            |
| `E_TYPE`                | type mismatch             | `TypeError`                     |
| `E_INVALID_ARG`         | invalid argument          | `TypeError`                     |
| `E_OUT_OF_RANGE`        | out of bounds             | `RangeError`                    |
| `E_MISSING_PROPERTY`    | property not found        | `ReferenceError`                |
| `E_IO`                  | IO error                  | `Error`                         |
| `E_PERMISSION_DENIED`   | permission denied         | `Error`                         |
| `E_NOT_FOUND`           | resource not found        | `Error`                         |
| `E_ALREADY_EXISTS`      | resource already exists   | `Error`                         |
| `E_ABORT`               | aborted                   | `AbortError`                    |
| `E_INVALID_STATE`       | invalid state             | `Error` / `InvalidStateError`   |
| `E_INVALID_DATA`        | data format error         | `Error`                         |
| `E_NOT_SUPPORTED`       | feature not supported     | `Error`                         |
| `E_STREAM`              | stream error              | `Error`                         |
| `E_COMPILE`             | compilation error         | `SyntaxError`                   |
| `E_INTERNAL`            | internal error            | `Error`                         |
| `E_ILLEGAL_CONSTRUCTOR` | illegal constructor       | `TypeError`                     |
| `E_JS_THROWN`           | wrapped JS thrown value   | `Error` (payload preserved)     |

Modules may define their own prefixed codes (`FS_*`, `NET_*`, `HTTP_*`, ...).

## Preserve JS-thrown values

When a value comes from JS (callback result, promise rejection, abort reason),
**re-throw it as-is** - don't wrap it, or the caller loses `cause`/identity.

```rust
let result = func.call(None, args)?;
if result.is_exception() {
    return Err(RongJSError::from_thrown_value(result));
}

// AbortController.abort(reason): preserve reason ===
if let Some(reason) = signal.reason() {
    return Err(RongJSError::from_thrown_value(reason));
}
return Err(HostError::aborted(None).into()); // no reason -> default AbortError
```

## Throwing vs no-throw APIs

Default: failures throw (good for network/IO/permissions/cancellation). For
frequent *expected* failures, return an explicit result object with `Ok(...)`
instead of `Err`, and name it `tryXxx` / `xxxOrNull`:

```rust
pub fn try_parse_json(ctx: &JSContext, text: String) -> JSResult<JSValue> {
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(v)  => Ok(rong::js_value!(ctx, { ok: true,  value: (v) })),
        Err(e) => Ok(rong::js_value!(ctx, { ok: false, error: (e.to_string()) })),
    }
}
```

Pick one style per function - don't mix "sometimes throws, sometimes returns null".

## Common mistakes

- Using `is_error()` to detect exceptions (it only means "is an Error object").
- Wrapping a JS-thrown value in a new `HostError` (loses the original payload).
- Mixing throw and no-throw in a single function.
