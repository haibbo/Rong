# Rong JavaScript Runtime - Type Reference (developer-facing)

## Runtime globals (mount points)

- `Rong` (filesystem + storage): `rong_types/src/global.ts`, `rong_types/src/fs.ts`, `rong_types/src/storage.ts`
- `process`: `rong_types/src/process.ts`
- `child_process`: `rong_types/src/child_process.ts`
- `path`: `rong_types/src/path.ts`
- `fetch` + Fetch-related globals: `rong_types/src/http.ts` (no `http` namespace, no `download`)
- Timers: `rong_types/src/timer.ts`
- `assert`: `rong_types/src/assert.ts`
- `atob`/`btoa`: `rong_types/src/global.ts` + `rong_types/src/encoding.ts`

Notes:

- Directory listing is `Rong.readDir(...)` and returns an **async iterator**.
- Storage is *not* `localStorage`-compatible. Use `await Rong.storage.open(path)` or `new Rong.Storage(path)`.

---

## File System (`Rong.*`)

Key APIs:

- `Rong.file(path) -> RongFile`
- `Rong.write(dest, data) -> Promise<number>`
- `await Rong.file(path).text() / .json() / .bytes() / .arrayBuffer()`
- `await Rong.file(path).open(options?) -> Promise<FileHandle>`
- `await Rong.file(path).writer(options?) -> Promise<FileSink>`
- `Rong.readDir(path) -> Promise<AsyncIterableIterator<DirEntry>>`
- `await Rong.file(path).stat() -> Promise<FileInfo>`
- `await Rong.file(path).lstat() -> Promise<FileInfo>`

Iteration example (async iterator):

```ts
const it = await Rong.readDir('/tmp');
for await (const entry of it) {
  console.log(entry.name);
}
```

---

## Storage (`Rong.Storage` / `Rong.storage.open`)

Open:

- `await Rong.storage.open(path, options?) -> Storage`
- `new Rong.Storage(path, options?) -> Storage`

Storage methods:

- `await storage.set(key, value)`
- `const value = await storage.get(key)`
- `await storage.delete(key)`
- `await storage.clear()`
- `const keys = await storage.list(prefix?)` (sync iterator)
- `const info = await storage.info()` (`{ currentSize, limitSize, keyCount }`)

---

## Process (`process`)

See `rong_types/src/process.ts` for the full surface.

---

## Child Process (`child_process`)

Core signatures:

- `child_process.spawn(command, args?, options?) -> ChildProcess`
- `await child_process.exec(command, options?) -> ExecResult`
- `await child_process.execFile(file, args?, options?) -> ExecResult`

---

## HTTP (`fetch`)

Rong provides the global `fetch(...)` API (and `Headers`/`Request`/`Response` classes registered by the runtime).

- There is **no** `http` namespace object.
- There is **no** `download(...)` JavaScript API.
- Default request timeout is **60000ms (60s)**.
- `RequestInit` does not include a `timeout` option.
- For per-request cancellation, use `AbortSignal`.

Rong extension:

- `Headers.getSetCookie(): string[]`

---

## Path (`path`)

Path manipulation utilities (Node.js compatible), see `rong_types/src/path.ts`.

```typescript
// Join paths
const joined = path.join('/foo', 'bar', 'baz');  // "/foo/bar/baz"

// Resolve to absolute path
const absolute = path.resolve('foo', 'bar');  // "/current/dir/foo/bar"

// Normalize path
const normalized = path.normalize('/foo/bar/../baz');  // "/foo/baz"

// Get basename
const base = path.basename('/foo/bar/file.txt');       // "file.txt"
const name = path.basename('/foo/bar/file.txt', '.txt');  // "file"

// Get directory name
const dir = path.dirname('/foo/bar/baz');  // "/foo/bar"

// Get extension
const ext = path.extname('file.txt');     // ".txt"
const ext2 = path.extname('archive.tar.gz');  // ".gz"

// Check if absolute
const isAbs = path.isAbsolute('/foo/bar');  // true

// Parse path
const parsed = path.parse('/home/user/file.txt');
console.log(parsed);
// {
//   root: '/',
//   dir: '/home/user',
//   base: 'file.txt',
//   ext: '.txt',
//   name: 'file'
// }

// Format path
const formatted = path.format({
  root: '/',
  dir: '/home/user',
  base: 'file.txt'
});  // "/home/user/file.txt"

// Platform-specific separators
console.log(path.sep);       // "/" on Unix, "\\" on Windows
console.log(path.delimiter); // ":" on Unix, ";" on Windows
```

---

## Timers

Callback timers (`setTimeout`/`setInterval`) and a promise-based `timers` namespace.

```typescript
// Set timeout (runs once)
const timeoutId = setTimeout(() => {
  console.log('Executed after 1 second');
}, 1000);

// Clear timeout
clearTimeout(timeoutId);

// Set interval (runs repeatedly)
const intervalId = setInterval(() => {
  console.log('Executed every 500ms');
}, 500);

// Clear interval
clearInterval(intervalId);

// Immediate execution (delay = 0)
setTimeout(() => {
  console.log('Executes immediately');
});
```

---

## Events / Web APIs

Rong implements a subset of Web/Node-style APIs (e.g. `EventTarget`, `EventEmitter`, `AbortController`).
For typing, prefer the concrete module files in `rong_types/src/` and the `lib.dom.d.ts` baseline.
