# Child Process

Create and manage child processes.

## exec — Run Command

```javascript
const result = await child_process.exec("ls -la");
result.stdout;   // string
result.stderr;   // string
result.code;     // exit code
```

### With options

```javascript
const result = await child_process.exec("echo $HOME", {
  cwd: "/tmp",
  env: { HOME: "/custom" },
});
```

## execFile — Run Executable

```javascript
const result = await child_process.execFile("node", ["--version"]);
result.stdout; // "v20.0.0\n"
```

## spawn — Streaming Child Process

```javascript
const child = child_process.spawn("cat", ["-n"]);

// Write to stdin
const writer = child.stdin.getWriter();
await writer.write(new TextEncoder().encode("hello\n"));
await writer.close();

// Read from stdout
for await (const chunk of child.stdout) {
  console.log(new TextDecoder().decode(chunk));
}

// Wait for exit
const exitCode = await child.wait();
```

### Properties

```javascript
child.pid;       // process ID
child.exitCode;  // exit code (null while running)
child.stdin;     // WritableStream<Uint8Array> | null
child.stdout;    // ReadableStream<Uint8Array> | null
child.stderr;    // ReadableStream<Uint8Array> | null
```

### Terminate

```javascript
child.kill();         // send SIGTERM
child.kill("SIGKILL"); // force kill
```

### Events

```javascript
child.on("exit", (code) => {
  console.log("exited:", code);
});
```

## SpawnOptions

```javascript
child_process.spawn("cmd", ["arg"], {
  cwd: "/path",
  env: { KEY: "value" },
  shell: true,
  timeout: 5000,
});
```

`spawn()` currently creates piped `stdin` / `stdout` / `stderr` streams automatically; stdio mode selection is not part of the JS API.
