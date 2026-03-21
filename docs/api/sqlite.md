# SQLite

Embedded SQLite database with synchronous API. Uses WAL mode by default.

## Quick Start

```javascript
const db = new Database("mydb.sqlite");

db.exec("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)");
db.run("INSERT INTO users (name, age) VALUES (?, ?)", ["Alice", 30]);

const rows = db.query("SELECT * FROM users WHERE age > ?", [18]);
console.log(rows); // [{ id: 1, name: "Alice", age: 30 }]

db.close();
```

## Opening a Database

```javascript
const db = new Database("mydb.sqlite");   // file-based
const db = new Database(":memory:");      // in-memory
const db = new Database();                // in-memory (default)
```

## Executing SQL

### db.exec(sql)

Execute one or more statements. No parameters, no return value. Use for DDL / schema setup.

```javascript
db.exec(`
  CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER);
  CREATE TABLE logs (id INTEGER PRIMARY KEY, msg TEXT);
`);
```

### db.run(sql, params?)

Execute a single statement. Returns `{ changes, lastInsertRowid }`.

```javascript
const result = db.run("INSERT INTO users (name, age) VALUES (?, ?)", ["Alice", 30]);
result.changes;         // 1
result.lastInsertRowid; // 1
```

### db.query(sql, params?)

Execute a query and return all rows as array of objects.

```javascript
const rows = db.query("SELECT * FROM users WHERE age > ?", [18]);
// [{ id: 1, name: "Alice", age: 30 }, ...]
```

## Prepared Statements

For repeated queries, prepare once and execute many times.

```javascript
const stmt = db.prepare("SELECT * FROM users WHERE age > ?");
```

### stmt.all(params?)

Return all matching rows as objects.

```javascript
const rows = stmt.all([18]);
// [{ id: 1, name: "Alice", age: 30 }, ...]
```

### stmt.get(params?)

Return the first matching row, or `null`.

```javascript
const user = stmt.get([30]);
// { id: 1, name: "Alice", age: 30 } or null
```

### stmt.values(params?)

Return rows as arrays of values (no column names).

```javascript
const rows = stmt.values([18]);
// [[1, "Alice", 30], [2, "Bob", 25]]
```

### stmt.run(params?)

Execute without returning rows. Returns `{ changes, lastInsertRowid }`.

```javascript
const insert = db.prepare("INSERT INTO users (name, age) VALUES (?, ?)");
insert.run(["Bob", 25]);
insert.run(["Charlie", 35]);
```

### stmt.finalize()

Mark the statement as done. Further calls will throw.

```javascript
stmt.finalize();
```

## Transactions

```javascript
db.transaction(() => {
  db.run("INSERT INTO users (name, age) VALUES (?, ?)", ["Alice", 30]);
  db.run("INSERT INTO logs (msg) VALUES (?)", ["user created"]);
});
```

If the callback throws, the transaction is rolled back. Otherwise it commits.

## Properties

```javascript
db.filename;      // database file path
db.inTransaction; // true if inside a transaction
```

## Parameter Types

| JavaScript | SQLite |
|------------|--------|
| `null` / `undefined` | NULL |
| `boolean` | INTEGER (0 or 1) |
| `number` (integer) | INTEGER |
| `number` (float) | REAL |
| `string` | TEXT |
| `ArrayBuffer` / `Uint8Array` | BLOB |

## Result Type Mapping

| SQLite | JavaScript |
|--------|------------|
| NULL | `null` |
| INTEGER | `number` |
| REAL | `number` |
| TEXT | `string` |
| BLOB | `ArrayBuffer` |
