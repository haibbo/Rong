# Rong Command APIs

Command execution APIs exposed on `globalThis.Rong`.

## `Rong.env`

`Rong.env` is the mutable environment object exposed on `Rong`.

```javascript
Rong.env.APP_MODE = "local";
console.log(Rong.env.APP_MODE); // "local"
```

## `Rong.argv` / `Rong.args`

```javascript
console.log(Rong.argv); // full argv
console.log(Rong.args); // argv.slice(2)
```

## `Rong.stdin` / `Rong.stdout` / `Rong.stderr`

`Rong.stdin` is a readable byte stream with the same helpers used by subprocess output streams:

```javascript
const text = await Rong.stdin.text();
```

`Rong.stdout` and `Rong.stderr` are runtime output handles:

```javascript
Rong.stdout.write("hello\n");
Rong.stderr.write(new TextEncoder().encode("warn\n"));
```

## `Rong.spawn()`

```javascript
const proc = Rong.spawn(["echo", "hello"]);
console.log(await proc.stdout.text()); // "hello\n"
console.log(await proc.exited); // 0
```

Accepts either a command array:

```javascript
const proc = Rong.spawn(["echo", "hello"], { cwd: "/tmp" });
```

Or an object form:

```javascript
const proc = Rong.spawn({
  cmd: ["echo", "hello"],
  cwd: "/tmp",
  timeout: 1000,
});
```

## `Rong.spawnSync()`

```javascript
const proc = Rong.spawnSync(["echo", "hello"]);
console.log(new TextDecoder().decode(proc.stdout)); // "hello\n"
```

## `Rong.$`

```javascript
const result = await Rong.$`echo ${"hello"}`.text();
console.log(result); // "hello\n"
```

Helpers:

- `.text()`
- `.json()`
- `.lines()`
- `.blob()`
- `.run()`
- `.env(values)`
- `.throws(value?)`
- `.quiet()`
- `.nothrow()`
- `.cwd(path)`

Default shell helpers on `Rong.$`:

- `Rong.$.cwd(path?)`
- `Rong.$.env(values?)`
- `Rong.$.throws(value?)`
- `Rong.$.nothrow()`
- `Rong.$.quiet()`
- `Rong.$.escape(value)`
