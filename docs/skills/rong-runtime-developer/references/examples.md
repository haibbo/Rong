# Rong JavaScript Examples

These examples are self-contained summaries of the repository examples under
`examples/javascript`.

## Available Examples

- `downloader.js`: streaming downloader that saves a response body directly to a
  file.
- `uploader.js`: streaming uploader that sends a local file as a PUT request
  body.
- `sse.js`: Server-Sent Events demo with formatted event output.

## Run Examples

From the repository root:

```bash
cargo run -p rong_cli -- examples/javascript/downloader.js https://example.com/file.txt downloaded.txt
cargo run -p rong_cli -- examples/javascript/uploader.js path/to/local/file.txt https://httpbin.org/upload
cargo run -p rong_cli -- examples/javascript/sse.js https://sse.dev/test 15
```

If the binary is already built:

```bash
./target/debug/rong examples/javascript/downloader.js https://example.com/file.txt downloaded.txt
```

## Arguments

- `downloader.js`: `<url> <output-filename>`
- `uploader.js`: `<file-path> <server-url>`
- `sse.js`: `[url] [duration-seconds]`

Scripts read these through `Rong.args`.

## Bytecode

```bash
cargo run -p rong_cli -- compile examples/javascript/downloader.js downloader.rong
cargo run -p rong_cli -- downloader.rong https://example.com/file.txt downloaded.txt
```

## Patterns To Reuse

- Use `fetch()` for HTTP.
- Use streams for large request/response bodies.
- Use `Rong.file(path)` for lazy file handles.
- Use `AbortController` or duration arguments for examples that should stop.
- Keep examples runnable from the repository root.
