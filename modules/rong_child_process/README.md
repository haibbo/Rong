# rong_child_process

Spawn and manage child processes from JavaScript.

## JS APIs

- `child_process.spawn(command, args?, options?)` — spawn a child process with streaming I/O
- `child_process.exec(command, options?)` — execute a shell command and capture output
- `child_process.execFile(file, args?, options?)` — execute a file directly without a shell
- `ChildProcess` — represents a running child process (extends `EventEmitter`)
  - `pid` — process ID
  - `exitCode` — exit code after process exits
  - `stdin` / `stdout` / `stderr` — I/O streams (when piped)
  - `wait()` — wait for the process to exit
  - `kill(signal?)` — send a signal to the process
  - `on('exit', callback)` — listen for process exit
