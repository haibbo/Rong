# rong_console

Implements the Web Console API with format string support.

## JS APIs

- `console.log(...args)` — log to stdout
- `console.info(...args)` — log informational message to stdout
- `console.debug(...args)` — log debug message to stdout
- `console.warn(...args)` — log warning to stderr
- `console.error(...args)` — log error to stderr
- `console.assert(condition, ...args)` — log assertion failures to stderr
- `console.dir(value, options?)` — inspect a value with optional depth/length limits
- `console.trace(...args)` — log a stack trace
- `console.time(label?)` / `console.timeLog(label?, ...args)` / `console.timeEnd(label?)` — timer helpers
- `console.count(label?)` / `console.countReset(label?)` — counters for repeated code paths
- `console.clear()` — clear the console

Format specifiers: `%s` (string), `%d`/`%i` (integer), `%f` (float), `%o` (object inspection).

Inspect output understands typed arrays, circular references, class instances, and errors with stack traces.
