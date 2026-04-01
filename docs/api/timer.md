# Timer

Timer functions for callback-style scheduling.

## Callback Style

### setTimeout / clearTimeout

```javascript
const id = setTimeout(() => {
  console.log("runs after 1 second");
}, 1000);

clearTimeout(id); // cancel
```

### setInterval / clearInterval

```javascript
const id = setInterval(() => {
  console.log("runs every second");
}, 1000);

clearInterval(id); // stop
```

## Async / Sync Waiting

### `Rong.sleep(delay?)`

Asynchronously waits for a number of milliseconds or until a target `Date`.

```javascript
await Rong.sleep(100);
await Rong.sleep(new Date(Date.now() + 500));
```

Use this on hot runtime paths, in request handling, and anywhere blocking the current JS thread would be undesirable.

### `Rong.sleepSync(delay?)`

Synchronously blocks the current JS thread for the given number of milliseconds.

```javascript
Rong.sleepSync(25);
```

Use this only in tests, short scripts, startup-time work, or other paths where blocking is acceptable.
