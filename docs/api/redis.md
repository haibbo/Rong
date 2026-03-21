# Redis

Async Redis client with strings, hashes, lists, sets, pub/sub, and raw commands.

## Quick Start

```javascript
const client = new RedisClient("redis://127.0.0.1:6379");
await client.connect();

await client.set("key", "value");
const val = await client.get("key"); // "value"

client.close();
```

## Connection

```javascript
// Explicit URL
const client = new RedisClient("redis://127.0.0.1:6379");

// From environment (REDIS_URL or VALKEY_URL)
const client = new RedisClient();

await client.connect();
client.connected; // true
client.close();
```

## Strings

```javascript
await client.set("name", "rong");
await client.get("name");           // "rong"
await client.del("name");           // 1
await client.exists("name");        // false
```

## Numeric Operations

```javascript
await client.set("counter", "0");
await client.incr("counter");       // 1
await client.incrBy("counter", 5);  // 6
await client.decr("counter");       // 5
await client.decrBy("counter", 3);  // 2
```

## TTL

```javascript
await client.set("session", "data");
await client.expire("session", 60);  // expires in 60 seconds
await client.ttl("session");         // remaining seconds
```

## Hashes

```javascript
await client.hset("user:1", "name", "Alice");
await client.hset("user:1", "age", "30");
await client.hget("user:1", "name");        // "Alice"
await client.hgetall("user:1");             // { name: "Alice", age: "30" }
await client.hkeys("user:1");              // ["name", "age"]
await client.hvals("user:1");              // ["Alice", "30"]
await client.hexists("user:1", "name");    // true
await client.hdel("user:1", "age");        // 1
```

## Lists

```javascript
await client.lpush("queue", "first");
await client.rpush("queue", "last");
await client.llen("queue");                // 2
await client.lrange("queue", 0, -1);       // ["first", "last"]
await client.lpop("queue");                // "first"
await client.rpop("queue");                // "last"
```

## Sets

```javascript
await client.sadd("tags", "rust");
await client.sadd("tags", "javascript");
await client.smembers("tags");             // ["rust", "javascript"]
await client.sismember("tags", "rust");    // true
await client.scard("tags");                // 2
await client.srem("tags", "rust");         // 1
```

## Pub/Sub

```javascript
// Subscriber
const sub = new RedisClient();
await sub.connect();
sub.on("message", (channel, message) => {
  console.log(`${channel}: ${message}`);
});
await sub.subscribe("notifications");

// Publisher
const pub = new RedisClient();
await pub.connect();
await pub.publish("notifications", "hello!");
```

## Raw Commands

```javascript
await client.send(["PING"]);                    // "PONG"
await client.send(["SET", "key", "value"]);
await client.send(["GET", "key"]);              // "value"
await client.send(["MGET", "k1", "k2", "k3"]); // ["v1", "v2", null]
```

## API Summary

| Method | Description |
|--------|-------------|
| `connect()` | Establish connection |
| `close()` | Close connection |
| `connected` | Whether connected |
| `set(key, value)` | Set string |
| `get(key)` | Get string |
| `del(key)` | Delete key |
| `exists(key)` | Key exists |
| `expire(key, seconds)` | Set expiration |
| `ttl(key)` | Get remaining TTL |
| `incr(key)` / `decr(key)` | Increment / decrement |
| `incrBy(key, n)` / `decrBy(key, n)` | Increment / decrement by value |
| `hset` / `hget` / `hgetall` / `hdel` / `hexists` / `hkeys` / `hvals` | Hash operations |
| `lpush` / `rpush` / `lpop` / `rpop` / `llen` / `lrange` | List operations |
| `sadd` / `srem` / `smembers` / `sismember` / `scard` | Set operations |
| `subscribe(channel)` / `publish(channel, message)` | Pub/sub |
| `send(args)` | Raw command |
