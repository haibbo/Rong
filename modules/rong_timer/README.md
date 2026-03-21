# rong_timer

Timer APIs including standard callbacks and promise-based alternatives.

## JS APIs

- `setTimeout(callback, delay?)` — execute a callback once after a delay; returns a timer ID
- `clearTimeout(id)` — cancel a timeout
- `setInterval(callback, delay?)` — execute a callback repeatedly at an interval; returns a timer ID
- `clearInterval(id)` — cancel an interval
- `timers.setTimeout(delay?)` — promise-based timeout, resolves with a timestamp
- `timers.setImmediate()` — promise-based immediate, resolves with a timestamp
- `timers.setInterval(delay?)` — async iterator that yields timestamps on each interval tick
