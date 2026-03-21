/**
 * Redis subscriber — real-time stats for pub/sub stress testing.
 *
 * Usage:
 *   rong tests/bench/redis_sub.js [channel] [reportIntervalMs]
 *
 * Pair with redis_pub.js in another terminal.
 * Press Ctrl+C to stop and print final report.
 *
 * Message format expected: JSON { seq, ts }
 */

const REDIS_URL =
  (typeof process !== "undefined" && process.env?.REDIS_URL) ||
  "redis://127.0.0.1:6379";

const args =
  typeof process !== "undefined" ? process.argv.slice(2) : [];
const CHANNEL = args[0] || "bench:pubsub";
const REPORT_INTERVAL = parseInt(args[1]) || 2000;

const client = new RedisClient(REDIS_URL);

// ── Stats ────────────────────────────────────────────────────────────

let totalReceived = 0;
let totalBytes = 0;
let windowReceived = 0;
let windowStart = Date.now();

// Latency tracking (ms)
let latencySum = 0;
let latencyMin = Infinity;
let latencyMax = 0;
let latencyCount = 0;

// Ordering & gaps
let lastSeq = -1;
let outOfOrder = 0;
let gaps = 0; // missed sequence numbers

function onMessage(raw) {
  totalReceived++;
  windowReceived++;
  totalBytes += raw.length;

  try {
    const msg = JSON.parse(raw);
    const now = Date.now();

    // Latency
    if (typeof msg.ts === "number") {
      const lat = now - msg.ts;
      latencySum += lat;
      latencyCount++;
      if (lat < latencyMin) latencyMin = lat;
      if (lat > latencyMax) latencyMax = lat;
    }

    // Ordering
    if (typeof msg.seq === "number") {
      if (lastSeq >= 0) {
        if (msg.seq <= lastSeq) outOfOrder++;
        else if (msg.seq > lastSeq + 1) gaps += msg.seq - lastSeq - 1;
      }
      lastSeq = msg.seq;
    }
  } catch {
    // Non-JSON message, just count it
  }
}

function printReport() {
  if (windowReceived === 0) {
    windowStart = Date.now();
    return;
  }

  const now = Date.now();
  const windowElapsed = (now - windowStart) / 1000;
  const windowRps = (windowReceived / windowElapsed).toFixed(0);
  const avgLat =
    latencyCount > 0 ? (latencySum / latencyCount).toFixed(1) : "—";
  const minLat = latencyMin === Infinity ? "—" : latencyMin.toFixed(0);
  const maxLat = latencyMax === 0 ? "—" : latencyMax.toFixed(0);
  const kbTotal = (totalBytes / 1024).toFixed(1);

  console.log(
    `[sub] ${windowRps} msg/s | total: ${totalReceived} (${kbTotal} KB) | ` +
      `latency: avg=${avgLat}ms min=${minLat}ms max=${maxLat}ms | ` +
      `gaps: ${gaps}, ooo: ${outOfOrder}`,
  );

  // Reset window
  windowReceived = 0;
  windowStart = now;
}

function printFinal() {
  console.log("\n── Final Report ──");
  console.log(`  Total received : ${totalReceived}`);
  console.log(`  Total bytes    : ${(totalBytes / 1024).toFixed(1)} KB`);
  if (latencyCount > 0) {
    console.log(
      `  Latency avg    : ${(latencySum / latencyCount).toFixed(1)} ms`,
    );
    console.log(`  Latency min    : ${latencyMin.toFixed(0)} ms`);
    console.log(`  Latency max    : ${latencyMax.toFixed(0)} ms`);
  }
  console.log(`  Gaps (missed)  : ${gaps}`);
  console.log(`  Out-of-order   : ${outOfOrder}`);
  if (lastSeq >= 0) {
    const expected = lastSeq + 1;
    const lossRate = (((expected - totalReceived) / expected) * 100).toFixed(2);
    console.log(
      `  Delivery       : ${totalReceived}/${expected} (${lossRate}% loss)`,
    );
  }
}

async function main() {
  console.log(`Subscribing to "${CHANNEL}" on ${REDIS_URL}`);
  console.log(`Report every ${REPORT_INTERVAL}ms. Ctrl+C to stop.\n`);

  const sub = await client.subscribe(CHANNEL);

  const timer = setInterval(printReport, REPORT_INTERVAL);
  let shuttingDown = false;

  if (typeof process !== "undefined") {
    process.on("SIGINT", () => {
      if (shuttingDown) return;
      shuttingDown = true;
      clearInterval(timer);
      sub.close();
      client.close();
      printFinal();
      process.exit(0);
    });
  }

  for await (const item of sub) {
    onMessage(item.message);
  }

  if (!shuttingDown) {
    clearInterval(timer);
    client.close();
    printFinal();
  }
}

main().catch((e) => {
  console.error("Sub error:", e.message || e);
  if (typeof process !== "undefined") process.exit(1);
});
