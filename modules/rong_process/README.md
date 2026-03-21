# rong_process

Provides the global `process` object with runtime and environment information.

## JS APIs

- `process.pid` — process ID
- `process.cwd()` — get current working directory
- `process.chdir(directory)` — change working directory
- `process.env` — environment variables (read/write)
- `process.platform` — OS platform (e.g., "darwin", "linux", "win32")
- `process.arch` — CPU architecture (e.g., "x64", "arm64")
- `process.version` — runtime version string
- `process.argv` — command-line arguments
- `process.exit(code?)` — exit the process
- `process.uptime()` — process uptime in seconds
- `process.hrtime(prev?)` — high-resolution time as `[seconds, nanoseconds]`
- `process.nextTick(callback, ...args)` — schedule a microtask
- `process.stdin` — standard input as a ReadableStream (with `isTTY`)
- `process.stdout` — standard output (with `write()` and `isTTY`)
- `process.stderr` — standard error (with `write()` and `isTTY`)
