# rong_http

HTTP client providing the standard Fetch API and Server-Sent Events.

## JS APIs

- `fetch(url, options?)` — global function for making HTTP requests
  - Supports `method`, `headers`, `body`, `signal`, `redirect`
  - Returns `Response` with `text()`, `json()`, `arrayBuffer()`, `blob()`, `formData()`, `body` stream
  - Default timeout: 60 seconds; use `AbortSignal` for per-request cancellation
- `Headers` — HTTP headers class
  - `get()`, `set()`, `append()`, `delete()`, `has()`, `forEach()`, `entries()`, `keys()`, `values()`, `getSetCookie()`
- `Request` — HTTP request class
  - `method`, `headers`, `url`, `signal`, `redirect`, `clone()`, plus body mixin methods
- `EventSource` — Server-Sent Events client
  - `new EventSource(url, options?)` — connect to an SSE endpoint
  - `addEventListener(type, listener)` — listen for events (`open`, `message`, `error`)
  - `removeEventListener(type, listener)` — remove a listener
  - `close()` — close the connection
  - `url`, `readyState`, `lastEventId`
  - Options: `headers`, `requestTimeoutMs`, `reconnect` (with `enabled`, `maxRetries`, `baseDelayMs`, `maxDelayMs`)
