# rong_event

Implements Web-standard events and a Node.js-style EventEmitter.

## JS APIs

- `Event` — base event class
  - `new Event(type, options?)` — create an event
  - `type` / `bubbles` / `cancelable` / `composed` — event properties
- `CustomEvent` — event with custom data (extends `Event`)
  - `new CustomEvent(type, options?)` — create with optional `detail`
  - `detail` — custom event data
- `EventTarget` — Web-standard event target
  - `addEventListener(type, listener, options?)` — add a listener
  - `removeEventListener(type, listener, options?)` — remove a listener
  - `dispatchEvent(event)` — dispatch an event
- `EventEmitter` — Node.js-style emitter (extends `EventTarget`)
  - `on(event, listener)` / `once(event, listener)` — add listeners
  - `off(event, listener)` / `removeListener(event, listener)` — remove listeners
  - `removeAllListeners(event?)` — remove all listeners
  - `prependListener(event, listener)` / `prependOnceListener(event, listener)` — add at front
  - `emit(event, ...args)` — emit an event
  - `eventNames()` — list registered event names
  - `listenerCount(event, listener?)` — count listeners
  - `setMaxListeners(n)` / `getMaxListeners()` — configure listener limit
