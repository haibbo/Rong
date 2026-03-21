# rong_buffer

Implements the Web Blob and File APIs for binary data handling.

## JS APIs

- `Blob` — immutable raw binary data
  - `new Blob(parts?, options?)` — create from arrays, strings, or other blobs
  - `size` — byte length
  - `type` — MIME type
  - `slice(start?, end?, contentType?)` — create a sub-blob
  - `arrayBuffer()` — read contents as `ArrayBuffer`
  - `text()` — read contents as UTF-8 string
  - `bytes()` — read contents as `Uint8Array`
- `File` — extends `Blob` with a filename and timestamp
  - `new File(bits, name, options?)` — create a named file blob
  - `name` — file name
  - `lastModified` — last modified timestamp in milliseconds
