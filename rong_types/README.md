# @rong/types

TypeScript type definitions for the Rong JavaScript runtime (globals injected by Rust).

## Scope

- Provides `.d.ts`/TS sources for the runtime surface so editors/TS can typecheck Rong scripts.
- Not a runtime polyfill: it does not provide implementations, only types.

## Runtime Export Map (high level)

Rong injects a small set of globals:

- `Rong` namespace: file system + storage
- Globals: `process`, `child_process`, `path`, `fetch`, `timers`, `assert`, `atob`, `btoa`

Rong also implements/extends a subset of Web APIs; the type package relies on TypeScript’s DOM libs for base types like `URL`, `ReadableStream`, `AbortController`, etc.

## Installation

```bash
npm install @rong/types
```

## Usage (typechecking only)

Add to your `tsconfig.json`:

```json
{
  "compilerOptions": {
    "types": ["@rong/types"]
  }
}
```

Notes:

- This enables global typings; you should not `import` runtime modules like `'child_process'` or `'http'` (those are Rong globals, not Node modules).
- Ensure your `tsconfig.json` `lib` includes `"DOM"` if you want DOM globals (e.g. `URL`, `ReadableStream`) to be typed.

## Accuracy notes (common gotchas)

- Storage is not `localStorage`-compatible. Use `await Rong.storage.open(path)` or `new Rong.Storage(path)`, then `set/get/delete/clear/list/info`.
- Directory listing is `Rong.readDir(...)` (async iterator), not `Rong.readdir(...)`.
- HTTP is the global `fetch(...)` API. There is no `http` namespace and no `download` JS API.

## Development

```bash
# Install dependencies
npm install

# Build
npm run build

# Watch mode
npm run watch
```
