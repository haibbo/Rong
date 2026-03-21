# Timer

Timer functions in both callback and Promise styles.

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

## Promise Style

Available via the `timers` namespace:

### timers.setTimeout(delay?)

```javascript
const timestamp = await timers.setTimeout(1000);
console.log("1 second later", timestamp);
```

### timers.setImmediate()

```javascript
const timestamp = await timers.setImmediate();
```

### timers.setInterval(delay?)

Returns an async iterator:

```javascript
for await (const timestamp of timers.setInterval(1000)) {
  console.log("tick", timestamp);
  if (shouldStop) break;
}
```
