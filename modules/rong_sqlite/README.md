# rong_sqlite

Embedded SQLite database with synchronous JS API. Exposed as global `SQLite`.
Uses `rusqlite` with bundled SQLite and prefers WAL for file-backed databases
when available.

## JS APIs

- `SQLite` — global SQLite database class
  - `new SQLite(filename?)` — open a database (defaults to `":memory:"`)
  - `exec(sql)` — execute one or more statements (DDL/schema)
  - `run(sql, params?)` — execute a statement, returns `{ changes, lastInsertRowid }`
  - `query(sql, params?)` — query rows as array of objects
  - `prepare(sql)` — create a prepared statement
  - `transaction(callback)` — run callback in a transaction (auto commit/rollback)
  - `close()` — close the connection
  - `filename` — database file path (getter)
  - `inTransaction` — whether inside a transaction (getter)
- `Statement` — prepared statement (via `db.prepare()`)
  - `all(params?)` — all matching rows as objects
  - `get(params?)` — first matching row, or `null`
  - `values(params?)` — all rows as value arrays
  - `run(params?)` — execute, returns `{ changes, lastInsertRowid }`
  - `finalize()` — release the statement
  - `sql` — the SQL text (getter)

Large SQLite integers and `lastInsertRowid` are returned as JavaScript `bigint` when they exceed the safe integer range.

For full API documentation, see [docs/api/sqlite.md](../../docs/api/sqlite.md).
