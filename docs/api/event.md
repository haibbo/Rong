# Event

Web-standard event system with Node.js EventEmitter support.

## EventTarget

```javascript
const target = new EventTarget();

target.addEventListener("click", (event) => {
  console.log(event.type); // "click"
});

target.dispatchEvent(new Event("click"));
```

### addEventListener Options

```javascript
target.addEventListener("click", handler, { once: true });
target.removeEventListener("click", handler);
```

`once` is supported. The `capture` flag is currently accepted for compatibility but not implemented.

## Event & CustomEvent

```javascript
const event = new Event("load", {
  bubbles: false,
  cancelable: true,
  composed: false,
});

const custom = new CustomEvent("message", {
  detail: { userId: 42 },
});
custom.detail; // { userId: 42 }
```

## EventEmitter (Node.js Style)

```javascript
const emitter = new EventEmitter();

emitter.on("data", (msg) => console.log(msg));
emitter.once("close", () => console.log("closed"));
emitter.emit("data", "hello");

emitter.off("data", handler);
emitter.removeAllListeners("data");
```

### Methods

| Method | Description |
|--------|-------------|
| `on(event, listener)` | Add listener |
| `once(event, listener)` | Add one-time listener |
| `off(event, listener)` | Remove listener |
| `removeListener(event, listener)` | Same as `off` |
| `removeAllListeners(event?)` | Remove all listeners |
| `prependListener(event, listener)` | Add to front of queue |
| `prependOnceListener(event, listener)` | Add one-time to front |
| `emit(event, ...args)` | Emit event |
| `eventNames()` | List registered event names |
| `listenerCount(event)` | Number of listeners |
| `setMaxListeners(n)` | Set max listener count |
| `getMaxListeners()` | Get max listener count |
