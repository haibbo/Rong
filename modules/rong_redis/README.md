# rong_redis

Async Redis client. Exposed as global `RedisClient`.

## JS APIs

- `RedisClient` — global Redis client class
  - `new RedisClient(url)` — create a client with an explicit Redis URL
  - `connect()` — explicitly connect (optional, commands auto-connect)
  - `close()` — close the connection
  - `connected` — whether a connection is currently held
- **String operations**
  - `set(key, value)` — set a key
  - `get(key)` — get a key (returns `null` if missing)
  - `del(key)` — delete a key
  - `exists(key)` — check if a key exists
  - `expire(key, seconds)` — set TTL
  - `ttl(key)` — get remaining TTL
- **Numeric operations**
  - `incr(key)` / `decr(key)` — increment / decrement
- **Hash operations**
  - `hset(key, field, value)` / `hget(key, field)` — single field
  - `hmset(key, [field, value, ...])` / `hmget(key, [field, ...])` — multiple fields
  - `hincrby(key, field, n)` / `hincrbyfloat(key, field, n)` — increment field
- **Set operations**
  - `sadd(key, member)` / `srem(key, member)` — add / remove
  - `sismember(key, member)` — check membership
  - `smembers(key)` — get all members
  - `srandmember(key)` / `spop(key)` — random member / pop
- **List operations**
  - `lpush(key, value)` / `rpush(key, value)` — push
  - `lpop(key)` / `rpop(key)` — pop
  - `lrange(key, start, stop)` — get range
  - `llen(key)` — get length
- **Pub/Sub**
  - `publish(channel, message)` — publish a message
  - `subscribe(channel, { signal? }?)` — returns a `RedisSubscription` async iterator
  - `RedisSubscription.close()` — explicitly close a subscription
  - `for await...of` with `break` closes the underlying subscription via iterator `return()`
- **Raw commands**
  - `send(command, args)` — execute any Redis command

## Rust API

- `RedisClient::new(url, namespace_prefix)` — create a pre-configured client from Rust, useful for environments that inject instances via a platform namespace instead of exposing the JS constructor.

To hide the JS constructor after init:

```rust
rong_redis::init(&ctx)?;
ctx.global().delete("RedisClient")?;
```
