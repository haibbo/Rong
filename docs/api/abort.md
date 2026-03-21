# AbortController & AbortSignal

Web-standard cancellation mechanism for aborting async operations.

## Basic Usage

```javascript
const controller = new AbortController();
const signal = controller.signal;

// Pass signal to async operations
fetch(url, { signal });

// Abort
controller.abort();
controller.abort("custom reason");
```

## AbortSignal

```javascript
signal.aborted;  // boolean
signal.reason;   // abort reason

signal.throwIfAborted(); // throws reason if aborted

signal.onabort = (event) => {
  console.log("aborted");
};
```

### Static Methods

```javascript
// Create already-aborted signal
const signal = AbortSignal.abort("reason");

// Auto-abort after timeout
const signal = AbortSignal.timeout(5000); // 5 seconds

// Abort when any signal fires
const signal = AbortSignal.any([signal1, signal2]);
```

## With fetch

```javascript
const controller = new AbortController();

setTimeout(() => controller.abort(), 5000);

try {
  const response = await fetch(url, { signal: controller.signal });
  const data = await response.json();
} catch (e) {
  if (e.name === "AbortError") {
    console.log("request cancelled");
  }
}
```
