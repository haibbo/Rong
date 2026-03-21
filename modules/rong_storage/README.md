# rong_storage

Key-value storage backed by a local database file.

## JS APIs

- `Rong.storage.open(path, options?)` — open a storage database, returns a `Storage` instance
- `new Rong.Storage(path, options?)` — construct a storage instance directly
  - Options: `maxKeySize`, `maxValueSize`, `maxDataSize`
- `Storage` instance methods:
  - `set(key, value)` — store a key-value pair
  - `get(key)` — retrieve a value by key
  - `delete(key)` — remove a key
  - `clear()` — remove all entries
  - `list(prefix?)` — list keys, optionally filtered by prefix
  - `info()` — get storage info (`currentSize`, `limitSize`, `keyCount`)
