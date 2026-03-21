/**
 * Redis client API — follows Bun's RedisClient API design.
 *
 * @example
 * ```typescript
 * const client = new RedisClient("redis://localhost:6379");
 * await client.set("key", "value");
 * const val = await client.get("key"); // "value"
 * client.close();
 * ```
 */

export type RedisReply =
  | null
  | string
  | number
  | bigint
  | boolean
  | RedisReply[]
  | { [key: string]: RedisReply };

export interface RedisSubscribeOptions {
  signal?: AbortSignal;
}

export interface RedisMessage {
  channel: string;
  message: string;
}

export interface RedisSubscription extends AsyncIterableIterator<RedisMessage> {
  readonly channel: string;
  close(): void;
}

export interface RedisClientInstance {
  /** Whether the client currently holds an open connection. */
  readonly connected: boolean;

  /** Explicitly connect to the Redis server. Commands auto-connect on first use. */
  connect(): Promise<void>;

  /** Close the connection. */
  close(): void;

  // ── String operations ──────────────────────────────────────────

  /** Set a string value. */
  set(key: string, value: string): Promise<string>;

  /** Get a string value, or `null` if the key does not exist. */
  get(key: string): Promise<string | null>;

  /** Delete a key. Returns the number of keys removed. */
  del(key: string): Promise<number>;

  /** Check if a key exists. */
  exists(key: string): Promise<boolean>;

  /** Set a timeout on a key (seconds). Returns `true` if the timeout was set. */
  expire(key: string, seconds: number): Promise<boolean>;

  /** Get the remaining time to live of a key (seconds). Returns -1 if no expiry, -2 if key does not exist. */
  ttl(key: string): Promise<number>;

  // ── Numeric operations ─────────────────────────────────────────

  /** Increment the integer value of a key by 1. Returns the new value. */
  incr(key: string): Promise<number>;

  /** Decrement the integer value of a key by 1. Returns the new value. */
  decr(key: string): Promise<number>;

  // ── Hash operations ────────────────────────────────────────────

  /** Set a hash field. Returns 1 if field is new, 0 if updated. */
  hset(key: string, field: string, value: string): Promise<number>;

  /** Get a hash field value, or `null` if the field does not exist. */
  hget(key: string, field: string): Promise<string | null>;

  /** Set multiple hash fields. `fields` is `[field, value, ...]`. */
  hmset(key: string, fields: string[]): Promise<string>;

  /** Get multiple hash field values. Returns an array with `null` for missing fields. */
  hmget(key: string, fields: string[]): Promise<(string | null)[]>;

  /** Increment a hash field by an integer amount. Returns the new value. */
  hincrby(key: string, field: string, increment: number): Promise<number>;

  /** Increment a hash field by a float amount. Returns the new value. */
  hincrbyfloat(key: string, field: string, increment: number): Promise<number>;

  // ── Set operations ─────────────────────────────────────────────

  /** Add a member to a set. Returns 1 if added, 0 if already present. */
  sadd(key: string, member: string): Promise<number>;

  /** Remove a member from a set. Returns 1 if removed, 0 if not present. */
  srem(key: string, member: string): Promise<number>;

  /** Check if a member is in a set. */
  sismember(key: string, member: string): Promise<boolean>;

  /** Get all members of a set. */
  smembers(key: string): Promise<string[]>;

  /** Get a random member from a set, or `null` if empty. */
  srandmember(key: string): Promise<string | null>;

  /** Remove and return a random member, or `null` if empty. */
  spop(key: string): Promise<string | null>;

  // ── List operations ────────────────────────────────────────────

  /** Push a value to the head of a list. Returns the new list length. */
  lpush(key: string, value: string): Promise<number>;

  /** Push a value to the tail of a list. Returns the new list length. */
  rpush(key: string, value: string): Promise<number>;

  /** Remove and return the first element, or `null` if empty. */
  lpop(key: string): Promise<string | null>;

  /** Remove and return the last element, or `null` if empty. */
  rpop(key: string): Promise<string | null>;

  /** Get a range of elements from a list. Use -1 for end of list. */
  lrange(key: string, start: number, stop: number): Promise<string[]>;

  /** Get the length of a list. */
  llen(key: string): Promise<number>;

  // ── Pub/Sub ────────────────────────────────────────────────────

  /** Publish a message to a channel. Returns the number of clients that received it. */
  publish(channel: string, message: string): Promise<number>;

  /**
   * Subscribe to a channel and receive messages through an async iterator.
   * `break` in `for await...of` closes the underlying subscription via `return()`.
   */
  subscribe(channel: string, options?: RedisSubscribeOptions): Promise<RedisSubscription>;

  // ── Raw command ────────────────────────────────────────────────

  /** Execute any Redis command. Integer replies may be returned as `bigint` when needed for precision. */
  send(command: string, args: string[]): Promise<RedisReply>;
}

export interface RedisClientConstructor {
  /**
   * Create a new Redis client.
   *
   * @param url - Redis URL (defaults to `REDIS_URL` / `VALKEY_URL` env var, or `redis://127.0.0.1:6379`)
   *
   * @example
   * ```typescript
   * const client = new RedisClient();
   * const client = new RedisClient("redis://localhost:6379");
   * const client = new RedisClient("redis://user:pass@host:6379/0");
   * ```
   */
  new (url?: string): RedisClientInstance;
}
