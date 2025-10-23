# JavaScript Examples

This directory contains JavaScript examples demonstrating how to use the Rong JavaScript engine.

## Available Examples

- `downloader.js` - Streaming downloader that saves response body directly to file (low memory)
- `uploader.js` - Streaming uploader that sends file as request body (PUT)

## Running Examples

Use the `rong_cli` tool with the `run` command to execute these examples:

```bash
# Download a file (streaming)
cargo run -p rong_cli -- run examples/javascript/downloader.js https://example.com/file.txt downloaded.txt

# Upload a file (streaming PUT)
cargo run -p rong_cli -- run examples/javascript/uploader.js path/to/local/file.txt https://httpbin.org/upload

# Or if you have built the binary
./target/debug/rong run examples/javascript/downloader.js https://example.com/file.txt downloaded.txt
```

## Command-line Arguments

The examples expect command-line arguments after the script name:

- For `downloader.js`: `<url> <output-filename>`
- For `uploader.js`: `<file-path> <server-url>` (server should accept raw body via PUT; script sets Content-Length + application/octet-stream)

These arguments are accessible in the scripts via the `Rong.args` array.

## Compiling JavaScript to Bytecode

You can also compile JavaScript files to bytecode for faster loading:

```bash
# Compile JavaScript to bytecode
cargo run -p rong_cli -- compile examples/javascript/downloader.js downloader.rong

# Run the compiled bytecode
cargo run -p rong_cli -- run downloader.rong https://example.com/file.txt downloaded.txt
```

## Notes

- Examples assume you run from the project root directory
- The streaming examples use ReadableStream/WritableStream and won't buffer the whole file in memory
