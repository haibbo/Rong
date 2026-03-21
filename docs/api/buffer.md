# Blob & File

Web-standard Blob and File implementation.

## Blob

```javascript
const blob = new Blob(["Hello ", "World"], { type: "text/plain" });
blob.size;  // 11
blob.type;  // "text/plain"
```

### Methods

```javascript
// Slice
const part = blob.slice(0, 5, "text/plain");

// Read contents
const text = await blob.text();
const buf = await blob.arrayBuffer();
const bytes = await blob.bytes();        // Uint8Array
```

## File

Extends Blob with filename and modification time.

```javascript
const file = new File(["content"], "hello.txt", {
  type: "text/plain",
  lastModified: Date.now(),
});

file.name;          // "hello.txt"
file.lastModified;  // timestamp
file.size;          // 7
```
