/**
 * Redis publisher — configurable high-throughput pub/sub stress test.
 *
 * Usage:
 *   rong tests/bench/redis_pub.js [channel] [totalMessages] [concurrency]
 *
 * Pair with redis_sub.js in another terminal.
 *
 * Message format: JSON { seq, ts }
 */

const REDIS_URL =
  (typeof process !== "undefined" && process.env?.REDIS_URL) ||
  "redis://127.0.0.1:6379";

const args =
  typeof process !== "undefined" ? process.argv.slice(2) : [];
const CHANNEL = args[0] || "bench:pubsub";
const TOTAL = parseInt(args[1]) || 10000;
const CONCURRENCY = parseInt(args[2]) || 50; // in-flight publishes

const client = new RedisClient(REDIS_URL);

async function main() {
  console.log(
    `Publishing ${TOTAL} messages to "${CHANNEL}" (concurrency=${CONCURRENCY})`,
  );
  console.log(`Redis: ${REDIS_URL}\n`);

  await client.connect();

  let sent = 0;
  let failed = 0;
  let seq = 0;

  const start = Date.now();
  let reportNext = start + 1000;

  // Publish in batches of CONCURRENCY to avoid unbounded in-flight
  while (seq < TOTAL) {
    const batchSize = Math.min(CONCURRENCY, TOTAL - seq);
    const batch = [];
    for (let i = 0; i < batchSize; i++) {
      const msg = JSON.stringify({ seq: seq++, ts: Date.now() });
      batch.push(
        client.publish(CHANNEL, msg).then(
          () => sent++,
          () => failed++,
        ),
      );
    }
    await Promise.all(batch);

    // Progress report every second
    const now = Date.now();
    if (now >= reportNext) {
      const elapsed = (now - start) / 1000;
      const rps = (sent / elapsed).toFixed(0);
      console.log(
        `[pub] ${sent}/${TOTAL} sent (${rps} msg/s), ${failed} failed`,
      );
      reportNext = now + 1000;
    }
  }

  const elapsed = Date.now() - start;
  const rps = ((sent / elapsed) * 1000).toFixed(0);

  console.log("\n── Publish Report ──");
  console.log(`  Total sent   : ${sent}`);
  console.log(`  Failed       : ${failed}`);
  console.log(`  Elapsed      : ${elapsed} ms`);
  console.log(`  Throughput   : ${rps} msg/s`);
  console.log(`  Success rate : ${((sent / TOTAL) * 100).toFixed(1)}%`);

  client.close();
}

main().catch((e) => {
  console.error("Pub error:", e.message || e);
  if (typeof process !== "undefined") process.exit(1);
});
