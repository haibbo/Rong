# Compression — Zstandard / Gzip

Compression helpers attached to `Rong`.

## `Rong.gzip(data, options?)`

Asynchronously compresses binary data with gzip.

```javascript
const compressed = await Rong.gzip(input, { level: 6 });
```

## `Rong.gzipSync(data, options?)`

Synchronously compresses binary data with gzip.

```javascript
const compressed = Rong.gzipSync(input, { level: 9 });
```

## `Rong.gunzipSync(data)`

Synchronously decompresses gzip-compressed bytes.

```javascript
const restored = Rong.gunzipSync(compressed);
```

## `Rong.gunzip(data)`

Asynchronously decompresses gzip-compressed bytes.

```javascript
const restored = await Rong.gunzip(compressed);
```

## Choosing Sync vs Async

- Prefer `Rong.zstdCompress()`, `Rong.zstdDecompress()`, `Rong.gzip()`, and `Rong.gunzip()` for large payloads or latency-sensitive runtime paths.
- Use the synchronous APIs when payloads are small and blocking is acceptable, such as startup code, tests, or one-shot scripts.
- Choose gzip when you need gzip-format interoperability. Choose zstd when you want better compression/runtime tradeoffs inside Rong-controlled environments.

## `Rong.zstdCompress(data, options?)`

Asynchronously compresses binary data with Zstandard.

```javascript
const input = new TextEncoder().encode("hello".repeat(100));
const compressed = await Rong.zstdCompress(input, { level: 6 });
```

## `Rong.zstdCompressSync(data, options?)`

Synchronously compresses binary data with Zstandard.

```javascript
const compressed = Rong.zstdCompressSync(input);
```

## `Rong.zstdDecompress(data)`

Asynchronously decompresses Zstandard-compressed bytes.

```javascript
const restored = await Rong.zstdDecompress(compressed);
```

## `Rong.zstdDecompressSync(data)`

Synchronously decompresses Zstandard-compressed bytes.

```javascript
const restored = Rong.zstdDecompressSync(compressed);
```

## Input

- `Uint8Array`
- other `TypedArray` views
- `ArrayBuffer`

## Options

### `level`

- integer from `1` to `22`
- default: `3`

### `gzip level`

- integer from `-1` to `9`
- default: `-1`
