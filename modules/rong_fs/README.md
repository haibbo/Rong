# rong_fs

File system operations for the Rong JavaScript runtime.

## Core API

Two entry points cover all file read/write needs:

```javascript
// Read — everything starts from Rong.file()
const f = Rong.file('/data.json');
await f.text();          // string
await f.json();          // parsed JSON
await f.bytes();         // Uint8Array
await f.arrayBuffer();   // ArrayBuffer
f.stream();              // ReadableStream
await f.exists();        // boolean
await f.delete();        // remove file
await f.stat();          // FileInfo
await f.lstat();         // FileInfo (no follow symlink)

// Write — one function handles all data types
await Rong.write('/out.txt', 'hello');              // string
await Rong.write('/out.bin', uint8array);            // TypedArray
await Rong.write('/out.bin', arrayBuffer);           // ArrayBuffer
await Rong.write('/copy.txt', Rong.file('/src.txt'));// copy (RongFile)
```

## Low-level FileHandle

For random access, seek, and truncate — open a handle:

```javascript
const handle = await Rong.file('/file.bin').open({ read: true, write: true });
await handle.seek(100, Rong.SeekMode.Start);
const buf = new ArrayBuffer(64);
await handle.read(buf);
await handle.truncate(200);
await handle.close(); // must close
```

`FileHandle` methods: `read()`, `write()`, `seek()`, `stat()`, `sync()`, `truncate()`, `close()`, `readable` (getter), `writable` (getter).

## Incremental Writing (FileSink)

For append or streaming writes:

```javascript
// Append to log file
const w = await Rong.file('/log.txt').writer({ append: true });
await w.write('line 1\n');
await w.write('line 2\n');
await w.flush();
await w.end();

// Overwrite (default, truncates existing)
const w2 = await Rong.file('/out.txt').writer();
await w2.write(new TextEncoder().encode('data'));
await w2.end();
```

`FileSink.write()` accepts `string | TypedArray | ArrayBuffer`. Other methods: `flush()`, `end()`.

## Directory & Path Operations

Top-level functions under `Rong`:

- `mkdir(path, options?)` — create directory, optionally `{ recursive: true }`
- `readDir(path)` — async iterator of `DirEntry` (`name`, `isFile`, `isDirectory`, `isSymlink`)
- `remove(path, options?)` — remove file or directory, optionally `{ recursive: true }`
- `rename(oldPath, newPath)` — rename/move
- `realPath(path)` — resolve to absolute canonical path
- `chdir(path)` — change working directory

## Symlink Operations

- `symlink(target, path)` — create symbolic link
- `readlink(path)` — read symlink target

## Permission & Timestamp Operations

- `chmod(path, mode)` — change permissions (Unix only)
- `chown(path, uid, gid)` — change ownership (Unix only)
- `utime(path, { accessed?, modified? })` — set access/modification times

## Constants

- `Rong.SeekMode.Start` (0), `Rong.SeekMode.Current` (1), `Rong.SeekMode.End` (2)
