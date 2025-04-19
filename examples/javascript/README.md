# JavaScript Examples

This directory contains JavaScript examples demonstrating how to use the Rong JavaScript engine.

## Available Examples

- `downloader.js` - A simple utility to download content from the web
- `uploader.js` - A tool to upload files to a web server

## Running Examples

Use the `rong_cli` tool with the `run` command to execute these examples:

```bash
# Download a file
cargo run -p rong_cli -- run examples/javascript/downloader.js https://example.com/file.txt downloaded.txt

# Upload a file
cargo run -p rong_cli -- run examples/javascript/uploader.js path/to/local/file.txt http://example.com/upload

# Or if you have built the binary
./target/debug/rong run examples/javascript/downloader.js https://example.com/file.txt downloaded.txt
```

## Command-line Arguments

The examples expect command-line arguments after the script name:

- For `downloader.js`: `<url> <output-filename>`
- For `uploader.js`: `<file-path> <server-url>`

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

- All examples should be run from the project root directory to ensure correct module resolution
- If you encounter "module not found" errors, check your working directory
