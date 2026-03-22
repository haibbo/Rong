# Redis

Async Redis client for JavaScript. Exposed as both global `RedisClient` and `Rong.RedisClient`.

## Quick Start

```javascript
const client = new RedisClient("redis://127.0.0.1:6379");
await client.connect();

await client.set("key", "value");
const val = await client.get("key"); // "value"

client.close();
```

## Connection

`RedisClient` requires an explicit Redis URL.

```javascript
const client = new RedisClient("redis://127.0.0.1:6379");

await client.connect();
client.connected; // true

client.close();
client.connected; // false
```

## Strings

```javascript
await client.set("name", "rong");
await client.get("name");     // "rong"
await client.del("name");     // 1
await client.exists("name");  // false
```

## Numeric Operations

```javascript
await client.set("counter", "0");
await client.incr("counter"); // 1
await client.decr("counter"); // 0
```

## TTL

```javascript
await client.set("session", "data");
await client.expire("session", 60); // expires in 60 seconds
await client.ttl("session");        // remaining seconds
```

## Hashes

```javascript
await client.hset("user:1", "name", "Alice");
await client.hset("user:1", "age", "30");

await client.hget("user:1", "name");              // "Alice"
await client.hmset("user:1", ["city", "HZ"]);     // "OK"
await client.hmget("user:1", ["name", "city"]);   // ["Alice", "HZ"]
await client.hincrby("user:1", "visits", 1);      // 1
await client.hincrbyfloat("user:1", "score", 0.5);
```

## Lists

```javascript
await client.lpush("queue", "first");
await client.rpush("queue", "last");
await client.llen("queue");          // 2
await client.lrange("queue", 0, -1); // ["first", "last"]
await client.lpop("queue");          // "first"
await client.rpop("queue");          // "last"
```

## Sets

```javascript
await client.sadd("tags", "rust");
await client.sadd("tags", "javascript");
await client.smembers("tags");            // ["rust", "javascript"]
await client.sismember("tags", "rust");   // true
await client.srandmember("tags");         // random member or null
await client.spop("tags");                // random member or null
await client.srem("tags", "rust");        // 1
```

## Pub/Sub

`subscribe()` returns an async iterator, not an event emitter.

```javascript
const pub = new RedisClient("redis://127.0.0.1:6379");
const subClient = new RedisClient("redis://127.0.0.1:6379");

const sub = await subClient.subscribe("notifications");

const pending = sub.next();
await pub.publish("notifications", "hello!");

const { value, done } = await pending;
console.log(done);           // false
console.log(value.channel);  // "notifications"
console.log(value.message);  // "hello!"

sub.close();
pub.close();
subClient.close();
```

You can also consume it with `for await...of`:

```javascript
const sub = await client.subscribe("notifications");

for await (const event of sub) {
  console.log(event.channel, event.message);
  break;
}
```

## Raw Commands

`send()` keeps raw Redis command semantics. It does not rewrite arguments.

```javascript
await client.send("PING", []);                         // "PONG"
await client.send("SET", ["key", "value"]);           // "OK"
await client.send("GET", ["key"]);                    // "value"
await client.send("MGET", ["k1", "k2", "k3"]);       // ["v1", "v2", null]
await client.send("INCRBY", ["counter", "10"]);      // bigint or number
```

## API Summary

| Method | Description |
|--------|-------------|
| `new RedisClient(url)` | Create a client with an explicit Redis URL |
| `connect()` | Establish a connection eagerly |
| `close()` | Close the connection and active subscriptions |
| `connected` | Whether the client currently holds an open connection |
| `set(key, value)` | Set string value |
| `get(key)` | Get string value or `null` |
| `del(key)` | Delete key |
| `exists(key)` | Check key presence |
| `expire(key, seconds)` | Set expiration |
| `ttl(key)` | Get remaining TTL |
| `incr(key)` / `decr(key)` | Increment / decrement |
| `hset(key, field, value)` | Set one hash field |
| `hget(key, field)` | Get one hash field |
| `hmset(key, fields)` | Set multiple hash fields from `[field, value, ...]` |
| `hmget(key, fields)` | Get multiple hash fields |
| `hincrby(key, field, n)` | Increment hash field by integer |
| `hincrbyfloat(key, field, n)` | Increment hash field by float |
| `sadd(key, member)` / `srem(key, member)` | Add / remove set member |
| `sismember(key, member)` | Check set membership |
| `smembers(key)` | Get all set members |
| `srandmember(key)` / `spop(key)` | Get or pop a random set member |
| `lpush(key, value)` / `rpush(key, value)` | Push to list |
| `lpop(key)` / `rpop(key)` | Pop from list |
| `lrange(key, start, stop)` | Get list range |
| `llen(key)` | Get list length |
| `publish(channel, message)` | Publish a message |
| `subscribe(channel, { signal? }?)` | Subscribe and return an async iterator |
| `RedisSubscription.close()` | Close a subscription explicitly |
| `send(command, args)` | Execute a raw Redis command |
