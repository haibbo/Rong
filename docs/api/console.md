# Console

Standard console output with format string support.

## Methods

```javascript
console.log("hello", { key: "value" });   // stdout
console.info("info message");              // stdout
console.warn("warning");                   // stderr
console.error("error");                    // stderr
console.debug("debug info");               // stdout, with DEBUG: prefix
console.clear();                           // clear screen (TTY)
```

## Formatting

Supports `%s` (string), `%d` / `%i` (integer), `%f` (float), `%o` / `%O` (object), `%%` (escaped %):

```javascript
console.log("name: %s, age: %d", "Alice", 30);
// name: Alice, age: 30
```
