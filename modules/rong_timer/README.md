# rong_timer

Timer APIs including standard callback timers plus Bun-aligned sleep helpers on `Rong`.

## JS APIs

- `setTimeout(callback, delay?)` — execute a callback once after a delay; returns a timer ID
- `clearTimeout(id)` — cancel a timeout
- `setInterval(callback, delay?)` — execute a callback repeatedly at an interval; returns a timer ID
- `clearInterval(id)` — cancel an interval
- `Rong.sleep(delay?)` — Bun-style async sleep; accepts milliseconds or a `Date`
- `Rong.sleepSync(delay?)` — Bun-style synchronous sleep; blocks the current thread
