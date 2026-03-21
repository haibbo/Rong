# rong_exception

Implements the Web DOMException API for structured error reporting.

## JS APIs

- `DOMException` — structured exception type (extends `Error`)
  - `new DOMException(message?, name?)` — create with a message and named error type
  - `name` — error name (e.g., `AbortError`, `TimeoutError`, `NotSupportedError`)
  - `message` — error message
  - `stack` — stack trace

Supported error names include: `AbortError`, `InvalidStateError`, `NetworkError`, `NotFoundError`, `NotSupportedError`, `QuotaExceededError`, `SecurityError`, `SyntaxError`, `TimeoutError`, `DataCloneError`, and others per the Web IDL spec.
