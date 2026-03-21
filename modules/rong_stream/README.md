# rong_stream

Web Streams API implementation providing globally available streaming primitives.

## JS APIs

- `ReadableStream` — readable stream of data
  - `getReader()` — acquire a reader for pull-based consumption
  - `pipeTo(writable)` — pipe to a writable stream
  - `pipeThrough(transform)` — pipe through a transform stream
- `WritableStream` — writable stream of data
  - `getWriter()` — acquire a writer for push-based writing
- `TransformStream` — transform stream for processing data between a readable and writable pair
  - `readable` — the transformed readable side
  - `writable` — the input writable side
