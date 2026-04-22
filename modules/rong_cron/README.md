# Rong Cron Module

In-process cron helpers for RongJS.

This module intentionally schedules jobs inside the current Rong process. It
does not register OS-level cron jobs.

## JS APIs

### `Rong.cron.parse(expression, relativeDate?)`

Alias: `Bun.cron.parse(expression, relativeDate?)`

Parse a five-field cron expression and return the next UTC `Date` after
`relativeDate`. If no future occurrence exists, returns `null`.

```js
const next = Rong.cron.parse("30 9 * * MON-FRI", new Date());
// Date | null
```

Arguments:

- `expression`: five-field cron expression: minute, hour, day-of-month, month,
  day-of-week.
- `relativeDate`: optional `Date` or epoch milliseconds. Defaults to now.

Supported conveniences:

- Nicknames: `@yearly`, `@annually`, `@monthly`, `@weekly`, `@daily`,
  `@midnight`, `@hourly`.
- Month and weekday names, including full names such as `January` and `Monday`.
- When both day-of-month and day-of-week are specified, matching uses OR
  semantics.

### `Rong.cron(schedule, handler)`

Alias: `Bun.cron(schedule, handler)`

Synchronously register an in-process cron job and return a `CronJob` handle.
The handler may be sync or async. If it returns a Promise, Rong waits for it to
settle before considering that tick complete. If another tick arrives while the
previous handler is still running, that tick is skipped instead of queued.

```js
const job = Rong.cron("* * * * *", function () {
  console.log(this.cron);
});

const asyncJob = Rong.cron("*/5 * * * *", async function () {
  await doWork();
});
```

`CronJob`:

- `job.cron`: normalized cron expression string.
- `job.stop()`: stop the job and return `job`.
- `job.ref()`: return `job`. Present for Bun API compatibility.
- `job.unref()`: return `job`. Present for Bun API compatibility.

### Unsupported API

OS-level cron registration is intentionally unsupported and throws a
`TypeError` when requested.
