# Storage

Persistent key-value storage.

## Open

```javascript
const store = await Rong.storage.open("mydb");
// or
const store = new Rong.Storage("./data.db");
```

## Operations

```javascript
// Write (values are JSON-serialized)
await store.set("user", { name: "Alice", age: 30 });

// Read
const user = await store.get("user");
// { name: "Alice", age: 30 }

// Delete
await store.delete("user");

// Clear all
await store.clear();
```

## List Keys

```javascript
// List all keys
for (const key of await store.list()) {
  console.log(key);
}

// Filter by prefix
for (const key of await store.list("user:")) {
  console.log(key);
}
```

## Info

```javascript
const info = await store.info();
```
