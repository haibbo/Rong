# File System

File system operations via `Rong.file()` and top-level utility functions.

## Quick Start

```javascript
const file = Rong.file("data.txt");

// Read
const text = await file.text();
const json = await file.json();
const bytes = await file.bytes();

// Write
await Rong.write("output.txt", "Hello World!");

// Check
const exists = await file.exists();
const info = await file.stat();

// Directory
await Rong.mkdir("logs", { recursive: true });
```

## Rong.file(path) — RongFile

Returns a lazy `RongFile` reference. This is a path-based file object, not an opened handle.
**No I/O until you call a method.**

```javascript
const file = Rong.file("data.json");
file.name; // "data.json"
```

> Direct `new RongFile()` is not allowed. Use `Rong.file(path)`.

### Reading

```javascript
const text = await file.text();           // string
const data = await file.json();           // parsed JSON
const bytes = await file.bytes();         // Uint8Array
const buf = await file.arrayBuffer();     // ArrayBuffer
```

### Streaming Read

```javascript
const stream = file.stream(); // ReadableStream<Uint8Array>

for await (const chunk of stream) {
  process(chunk);
}
```

### File Info

```javascript
await file.exists();  // boolean
await file.stat();    // FileInfo
await file.lstat();   // FileInfo (no symlink follow)
```

### Delete

```javascript
await file.delete();
```

### Low-Level: FileHandle

For random access read/write, open a `FileHandle`:

```javascript
const handle = await file.open({ read: true, write: true, create: true });

// Read into buffer
const buf = new ArrayBuffer(1024);
const bytesRead = await handle.read(buf); // number | null

// Write
const written = await handle.write(data);

// Seek
await handle.seek(0, Rong.SeekMode.Start);
await handle.seek(-10, Rong.SeekMode.End);
await handle.seek(5, Rong.SeekMode.Current);

// Other
await handle.truncate(100);
await handle.sync();
await handle.close();

// Stream access
handle.readable; // ReadableStream<Uint8Array>
handle.writable; // WritableStream<Uint8Array>
```

> Direct `new FileHandle()` is not allowed. Use `Rong.file(path).open()`.

### Streaming Write: FileSink

```javascript
const sink = await file.writer();
await sink.write("line 1\n");
await sink.write("line 2\n");
await sink.write(new Uint8Array([0x0a]));
await sink.flush();
await sink.end();
```

`file.writer()` is write-only. It truncates by default; use `{ append: true }` to append.

> Direct `new FileSink()` is not allowed. Use `Rong.file(path).writer()`.

---

## Rong.write(dest, data)

One-shot write helper. It overwrites the destination by default.

- `dest`: string path or `RongFile`
- `data`: string, `TypedArray`, `ArrayBuffer`, or `RongFile` (copy)

```javascript
await Rong.write("file.txt", "string content");
await Rong.write("file.txt", new Uint8Array([1, 2, 3]));
await Rong.write("file.txt", arrayBuffer);
await Rong.write(Rong.file("file.txt"), "hello");
await Rong.write("copy.txt", Rong.file("source.txt")); // file copy
```

Returns bytes written.

---

## Directory Operations

### Rong.mkdir(path, options?)

```javascript
await Rong.mkdir("a/b/c", { recursive: true });
```

### Rong.readDir(path)

Returns an async iterator of `DirEntry`:

```javascript
for await (const entry of await Rong.readDir(".")) {
  console.log(entry.name);        // "file.txt"
  console.log(entry.isFile);      // true
  console.log(entry.isDirectory); // false
  console.log(entry.isSymlink);   // false
}
```

### Rong.remove(path, options?)

```javascript
await Rong.remove("dir", { recursive: true });
```

### Rong.chdir(path)

```javascript
await Rong.chdir("/tmp");
```

---

## File Operations

| Function | Description |
|----------|-------------|
| `Rong.rename(old, new)` | Rename / move |
| `Rong.symlink(target, link)` | Create symlink |
| `Rong.readlink(path)` | Read link target |
| `Rong.realPath(path)` | Resolve real path |

## Permissions (Unix)

```javascript
await Rong.chmod("script.sh", 0o755);
await Rong.chown("file.txt", uid, gid);
await Rong.utime("file.txt", { modified: Date.now() });
```

---

## FileInfo

Returned by `file.stat()` and `handle.stat()`:

```javascript
const info = await Rong.file("data.txt").stat();
info.size;        // bytes
info.isFile;      // boolean
info.isDirectory; // boolean
info.isSymlink;   // boolean
info.modified;    // ms since epoch, or undefined
info.accessed;    // ms since epoch, or undefined
info.created;     // ms since epoch, or undefined
info.mode;        // Unix permission bits, or undefined
```

## SeekMode

```javascript
Rong.SeekMode.Start;   // 0 — from beginning
Rong.SeekMode.Current; // 1 — from current position
Rong.SeekMode.End;     // 2 — from end
```
