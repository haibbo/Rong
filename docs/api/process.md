# Process

Global `process` object for process info and control, compatible with Node.js API.

## Properties

```javascript
process.pid;       // process ID
process.platform;  // "darwin" | "linux" | "win32"
process.arch;      // CPU architecture
process.version;   // version string
process.argv;      // command-line arguments array
process.env;       // environment variables object
```

## Methods

```javascript
process.cwd();                 // current working directory
process.chdir("/tmp");         // change directory
process.exit(0);               // exit process
process.uptime();              // uptime in seconds
process.hrtime();              // high-resolution time [seconds, nanoseconds]
process.hrtime(prev);          // diff from previous
process.nextTick(() => {});    // execute on next tick
```

## Standard I/O

```javascript
// Write
process.stdout.write("hello");
process.stderr.write("error");

// TTY detection
process.stdin.isTTY;   // boolean
process.stdout.isTTY;  // boolean
process.stderr.isTTY;  // boolean

// stdin is a ReadableStream
for await (const chunk of process.stdin) {
  console.log(chunk);
}
```

## Events

```javascript
process.on("exit", (code) => {
  console.log("exit code:", code);
});
```
