# Rong Public API Index

Installed skills include `references/api-*.md` generated from `docs/api/*.md`
so an agent can work without loading repository docs. In this repository source
tree, read the matching `docs/api/<name>.md` file. Load only the files needed
for the current task.

## Core Web-Like APIs

- `api-http.md`: `fetch`, `Request`, `Response`, `Headers`, `FormData`, SSE.
- `api-stream.md`: `ReadableStream`, `WritableStream`, compression streams,
  async iteration, piping, teeing.
- `api-url.md`: `URL`, `URLSearchParams`.
- `api-abort.md`: `AbortController`, `AbortSignal`, timeouts, signal
  composition.
- `api-event.md`: `EventTarget`, `Event`, `CustomEvent`, `EventEmitter`.
- `api-buffer.md`: `Blob`, `File`.
- `api-encoding.md`: `TextEncoder`, `TextDecoder`, `btoa`, `atob`.
- `api-worker.md`: web workers and message passing.
- `api-console.md`: console methods, formatting, inspection.
- `api-exception.md`: `DOMException`.
- `api-assert.md`: assertion helpers.

## Rong Host APIs

- `api-command.md`: `Rong.env`, `Rong.argv`, `Rong.args`, stdin/stdout/stderr,
  `Rong.spawn`, `Rong.spawnSync`, and `Rong.$`.
- `api-fs.md`: `Rong.file`, `Rong.write`, directories, file handles, streaming
  file I/O.
- `api-timer.md`: `setTimeout`, `setInterval`, `Rong.sleep`, `Rong.sleepSync`.
- `api-compression.md`: gzip and zstd sync/async APIs.
- `api-storage.md`: persistent key-value `Storage`.
- `api-sqlite.md`: embedded SQLite.
- `api-redis.md`: async Redis client.
- `api-s3.md`: S3-compatible object storage client.

## Common Combinations

- HTTP file downloader: `api-http.md`, `api-stream.md`, `api-fs.md`,
  `api-abort.md`.
- Upload local file: `api-fs.md`, `api-http.md`, `api-stream.md`,
  `api-buffer.md`.
- Local data processing: `api-fs.md`, `api-sqlite.md`, `api-encoding.md`,
  `api-compression.md`.
- Worker pipeline: `api-worker.md`, `api-event.md`, plus task-specific APIs.
- CLI automation: `api-command.md`, `api-fs.md`, `api-encoding.md`,
  `api-console.md`.
