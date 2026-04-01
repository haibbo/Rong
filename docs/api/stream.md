# Streams — ReadableStream / WritableStream / CompressionStream / DecompressionStream

Web Streams API with async iteration, piping, and teeing.

## ReadableStream

### Create

```javascript
// From underlying source
const stream = new ReadableStream({
  start(controller) {
    controller.enqueue(new Uint8Array([1, 2, 3]));
    controller.close();
  },
});

// From fetch response
const response = await fetch(url);
const reader = response.body.getReader();
```

### Read

```javascript
// Reader mode
const reader = stream.getReader();
while (true) {
  const { value, done } = await reader.read();
  if (done) break;
  console.log(value); // Uint8Array chunk
}

// Async iteration (recommended)
for await (const chunk of stream) {
  console.log(chunk);
}
```

### Pipe

```javascript
await readable.pipeTo(writable);

// With options
await readable.pipeTo(writable, {
  preventClose: false,
  preventAbort: false,
  preventCancel: false,
  signal: controller.signal,
});

const transformed = readable.pipeThrough(new CompressionStream("gzip"));
```

### Tee

```javascript
const [branch1, branch2] = stream.tee();
```

### Properties & Methods

| Method / Property | Description |
|-------------------|-------------|
| `locked` | Whether locked by a reader |
| `getReader()` | Acquire reader (locks the stream) |
| `cancel(reason?)` | Cancel the stream |
| `pipeTo(dest, options?)` | Pipe to WritableStream |
| `pipeThrough(transform, options?)` | Pipe through an object exposing `readable` and `writable` |
| `tee()` | Split into two streams |
| `[Symbol.asyncIterator]` | Supports `for await...of` |

---

## WritableStream

### Create

```javascript
const writable = new WritableStream({
  write(chunk) {
    console.log("received:", chunk);
  },
  close() {
    console.log("stream closed");
  },
  abort(reason) {
    console.log("aborted:", reason);
  },
});
```

### Write

```javascript
const writer = writable.getWriter();
await writer.write(new Uint8Array([1, 2, 3]));
await writer.write(new Uint8Array([4, 5, 6]));
await writer.close();
```

### Properties & Methods

| Method / Property | Description |
|-------------------|-------------|
| `locked` | Whether locked by a writer |
| `getWriter()` | Acquire writer (locks the stream) |
| `abort(reason?)` | Abort the stream |

### Writer Methods

| Method | Description |
|--------|-------------|
| `write(chunk)` | Write a chunk |
| `close()` | Close the stream |
| `abort(reason?)` | Abort the stream |
| `releaseLock()` | Release the lock |

---

## Common Patterns

### Download and process

```javascript
const response = await fetch("https://example.com/large-file");
for await (const chunk of response.body) {
  process(chunk);
}
```

### Pipe to file

```javascript
const response = await fetch(sourceUrl);
const writable = file.writer();
await response.body.pipeTo(writable);
```

---

## `CompressionStream`

Supported formats:

- `"gzip"`
- `"deflate"`
- `"deflate-raw"`

```javascript
const compressed = source.pipeThrough(new CompressionStream("gzip"));
```

### Properties

| Method / Property | Description |
|-------------------|-------------|
| `readable` | Compressed output `ReadableStream` |
| `writable` | Input `WritableStream` |

---

## `DecompressionStream`

Supported formats:

- `"gzip"`
- `"deflate"`
- `"deflate-raw"`

```javascript
const decompressed = source.pipeThrough(new DecompressionStream("gzip"));
```

### Properties

| Method / Property | Description |
|-------------------|-------------|
| `readable` | Decompressed output `ReadableStream` |
| `writable` | Input `WritableStream` |
