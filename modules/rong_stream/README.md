# rong_stream

Web Streams API implementation providing globally available streaming primitives.

## JS APIs

- `ReadableStream` — readable stream of data
  - `getReader()` — acquire a reader for pull-based consumption
  - `pipeTo(writable)` — pipe to a writable stream
  - `pipeThrough(transform)` — pipe through an object exposing `readable` and `writable`
- `WritableStream` — writable stream of data
  - `getWriter()` — acquire a writer for push-based writing
- `CompressionStream` — transform-like stream for `gzip`, `deflate`, and `deflate-raw`
  - `readable` — compressed output stream
  - `writable` — uncompressed input stream
- `DecompressionStream` — transform-like stream for `gzip`, `deflate`, and `deflate-raw`
  - `readable` — decompressed output stream
  - `writable` — compressed input stream
