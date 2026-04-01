# rong_compression

Compression utilities attached to the `Rong` namespace.

## JS APIs

- `Rong.zstdCompress(data, options?)` - asynchronously compress bytes with Zstandard
- `Rong.zstdCompressSync(data, options?)` - synchronously compress bytes with Zstandard
- `Rong.zstdDecompress(data)` - asynchronously decompress Zstandard bytes
- `Rong.zstdDecompressSync(data)` - synchronously decompress Zstandard bytes
- `Rong.gzip(data, options?)` - asynchronously compress bytes with gzip
- `Rong.gzipSync(data, options?)` - synchronously compress bytes with gzip
- `Rong.gunzip(data)` - asynchronously decompress gzip bytes
- `Rong.gunzipSync(data)` - synchronously decompress gzip bytes

## When To Use Sync vs Async

- Use `zstdCompress()` / `zstdDecompress()` and `gzip()` / `gunzip()` when payloads may be large or when you are inside request handling, long-running services, workers, or other latency-sensitive code paths.
- Use the synchronous variants for short scripts, startup-time preprocessing, tests, build steps, and small payloads where blocking the current thread is acceptable.
- Prefer `gzip()` / `gunzip()` over the sync variants when gzip compatibility is required but the work sits on a hot runtime path.

### Input

- `Uint8Array`
- any other `TypedArray`
- `ArrayBuffer`

### Options

- `level` - compression level from `1` to `22`, default `3`

### Gzip Options

- `level` - compression level from `-1` to `9`, default `-1`
