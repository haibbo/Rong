/**
 * Redis stress test — validates all JS APIs under load with success tracking.
 *
 * Requires a running Redis server (default: redis://127.0.0.1:6379).
 * Override with REDIS_URL environment variable.
 *
 * Usage:
 *   rong tests/bench/redis.js
 */

const REDIS_URL =
  (typeof process !== "undefined" && process.env?.REDIS_URL) ||
  "redis://127.0.0.1:6379";

const client = new Rong.RedisClient(REDIS_URL);

// ── Helpers ──────────────────────────────────────────────────────────

function assert(cond, msg) {
  if (!cond) throw new Error("ASSERT: " + msg);
}

function assertEqual(a, b, label) {
  if (a !== b) throw new Error(`${label}: expected ${b}, got ${a}`);
}

async function bench(label, iterations, fn) {
  let ok = 0;
  let fail = 0;
  const start = Date.now();
  for (let i = 0; i < iterations; i++) {
    try {
      await fn(i);
      ok++;
    } catch {
      fail++;
    }
  }
  const elapsed = Date.now() - start;
  const opsPerSec = ((iterations / elapsed) * 1000).toFixed(1);
  const rate = ((ok / iterations) * 100).toFixed(1);
  console.log(
    `  ${label}: ${iterations} ops in ${elapsed}ms (${opsPerSec} ops/s) — ${rate}% ok, ${fail} failed`,
  );
  return { elapsed, opsPerSec: parseFloat(opsPerSec), ok, fail };
}

// ── Test suites ──────────────────────────────────────────────────────

async function testStrings() {
  console.log("\n[strings]");

  // Correctness
  await client.set("stress:key", "hello");
  assertEqual(await client.get("stress:key"), "hello", "get");
  assertEqual(await client.exists("stress:key"), true, "exists");
  assertEqual(await client.del("stress:key"), 1, "del");
  assertEqual(await client.get("stress:key"), null, "get after del");

  // Throughput with verification
  const setResult = await bench("SET", 10000, (i) =>
    client.set(`bench:str:${i}`, `val-${i}`),
  );
  assert(setResult.fail === 0, `SET had ${setResult.fail} failures`);

  let getOk = 0;
  const getResult = await bench("GET (verify)", 10000, async (i) => {
    const v = await client.get(`bench:str:${i}`);
    if (v === `val-${i}`) getOk++;
    else throw new Error(`mismatch at ${i}: got ${v}`);
  });
  assert(getResult.fail === 0, `GET had ${getResult.fail} failures`);
  assertEqual(getOk, 10000, "all GET values correct");

  await bench("DEL", 10000, (i) => client.del(`bench:str:${i}`));
}

async function testNumeric() {
  console.log("\n[numeric]");

  await client.del("stress:counter");
  const r = await bench("INCR", 10000, () => client.incr("stress:counter"));
  assert(r.fail === 0, `INCR had ${r.fail} failures`);
  const val = await client.get("stress:counter");
  assertEqual(val, "10000", "counter value after 10k incr");
  await client.del("stress:counter");
}

async function testHashes() {
  console.log("\n[hashes]");

  await client.del("stress:hash");
  await client.hmset("stress:hash", ["f1", "v1", "f2", "v2", "f3", "v3"]);
  const r = await client.hmget("stress:hash", ["f1", "f2", "f3"]);
  assertEqual(r[0], "v1", "hmget[0]");
  assertEqual(r[1], "v2", "hmget[1]");
  assertEqual(r[2], "v3", "hmget[2]");

  const hsetR = await bench("HSET", 5000, (i) =>
    client.hset("stress:hash", `field-${i}`, `value-${i}`),
  );
  assert(hsetR.fail === 0, `HSET had ${hsetR.fail} failures`);

  let hgetOk = 0;
  const hgetR = await bench("HGET (verify)", 5000, async (i) => {
    const v = await client.hget("stress:hash", `field-${i}`);
    if (v === `value-${i}`) hgetOk++;
    else throw new Error(`mismatch at ${i}: got ${v}`);
  });
  assert(hgetR.fail === 0, `HGET had ${hgetR.fail} failures`);
  assertEqual(hgetOk, 5000, "all HGET values correct");

  await client.del("stress:hash");
}

async function testSets() {
  console.log("\n[sets]");

  await client.del("stress:set");
  const addR = await bench("SADD", 5000, (i) =>
    client.sadd("stress:set", `m-${i}`),
  );
  assert(addR.fail === 0, `SADD had ${addR.fail} failures`);

  const members = await client.smembers("stress:set");
  assertEqual(members.length, 5000, "set size");

  assertEqual(await client.sismember("stress:set", "m-0"), true, "sismember");
  assertEqual(
    await client.sismember("stress:set", "m-99999"),
    false,
    "sismember false",
  );

  await client.del("stress:set");
}

async function testLists() {
  console.log("\n[lists]");

  await client.del("stress:list");
  const pushR = await bench("RPUSH", 5000, (i) =>
    client.rpush("stress:list", `item-${i}`),
  );
  assert(pushR.fail === 0, `RPUSH had ${pushR.fail} failures`);
  assertEqual(await client.llen("stress:list"), 5000, "list length");

  const range = await client.lrange("stress:list", 0, 4);
  assertEqual(range.length, 5, "lrange length");
  assertEqual(range[0], "item-0", "lrange[0]");

  const popR = await bench("LPOP", 5000, () => client.lpop("stress:list"));
  assert(popR.fail === 0, `LPOP had ${popR.fail} failures`);
  assertEqual(await client.llen("stress:list"), 0, "list empty after pop");
}

async function testTTL() {
  console.log("\n[ttl]");

  await client.set("stress:ttl", "expiring");
  await client.expire("stress:ttl", 60);
  const ttl = await client.ttl("stress:ttl");
  assert(ttl > 0 && ttl <= 60, `TTL in range: got ${ttl}`);
  assertEqual(await client.ttl("stress:noexist"), -2, "ttl nonexistent");
  await client.del("stress:ttl");
}

async function testRawSend() {
  console.log("\n[raw send]");

  const r = await bench("send SET+GET", 5000, async (i) => {
    await client.send("SET", [`bench:raw:${i}`, `v${i}`]);
    const v = await client.send("GET", [`bench:raw:${i}`]);
    if (v !== `v${i}`) throw new Error(`raw mismatch at ${i}`);
  });
  assert(r.fail === 0, `raw send had ${r.fail} failures`);

  // Cleanup
  for (let i = 0; i < 5000; i++) {
    await client.del(`bench:raw:${i}`);
  }
}

async function testPubSub() {
  console.log("\n[pub/sub — in-process sanity check]");
  console.log("  (For real stress test, use tests/bench/redis_pub.js + redis_sub.js in separate terminals)");

  const subscriber = new Rong.RedisClient(REDIS_URL);
  const publisher = new Rong.RedisClient(REDIS_URL);

  // ── Correctness ────────────────────────────────────────────────
  const sub = await subscriber.subscribe("stress:ch");
  await new Promise((r) => setTimeout(r, 200));

  const first = sub.next();
  await publisher.publish("stress:ch", "ping");
  const received = await first;
  assertEqual(received.done, false, "received a message");
  assertEqual(received.value.message, "ping", "correct message");
  assertEqual(received.value.channel, "stress:ch", "correct channel");

  // ── Close stops delivery ───────────────────────────────────────
  sub.close();
  await new Promise((r) => setTimeout(r, 100));
  await publisher.publish("stress:ch", "after-unsub");
  const done = await sub.next();
  assertEqual(done.done, true, "subscription is closed");

  console.log("  subscribe/close: OK");

  subscriber.close();
  publisher.close();
}

// ── Main ─────────────────────────────────────────────────────────────

async function main() {
  console.log(`Redis stress test — ${REDIS_URL}`);
  await client.connect();
  console.log("Connected.");

  await testStrings();
  await testNumeric();
  await testHashes();
  await testSets();
  await testLists();
  await testTTL();
  await testRawSend();
  await testPubSub();

  client.close();
  console.log("\nAll stress tests passed.");
}

main().catch((e) => {
  console.error("FAILED:", e.message || e);
  if (typeof process !== "undefined") process.exit(1);
});
