# Console

Standard console output with format string support.

## Methods

```javascript
console.log("hello", { key: "value" });   // stdout
console.info("info message");              // stdout
console.warn("warning");                   // stderr
console.error("error");                    // stderr
console.debug("debug info");               // stdout, with DEBUG: prefix
console.assert(condition, "message");      // stderr on failure
console.dir(value, { depth: 2 });          // inspected output
console.trace("marker");                   // stack trace
console.time("db");                        // start timer
console.timeLog("db", "query");            // log elapsed time
console.timeEnd("db");                     // log and stop timer
console.count("requests");                 // requests: 1, 2, 3...
console.countReset("requests");            // reset counter
console.clear();                           // clear screen (TTY)
```

## Formatting

Supports `%s` (string), `%d` / `%i` (integer), `%f` (float), `%o` / `%O` (object), `%%` (escaped %):

```javascript
console.log("name: %s, age: %d", "Alice", 30);
// name: Alice, age: 30
```

## Inspection

Inspection output is tuned for runtime debugging:

- typed arrays render as `Uint8Array(3) [ ... ]`
- circular references render as `[Circular]`
- class instances keep their constructor name, for example `User {id: 1}`
- errors render with stack traces without duplicating the headline
