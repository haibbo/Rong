// Server-Sent Events demo
// Usage: rong sse.js [url] [duration-seconds]

const args = Rong.args;
const url = args[1] || "https://sse.dev/test";
const durationSeconds = Number(args[2] || 10);
const durationMs = Number.isFinite(durationSeconds) && durationSeconds > 0
  ? Math.floor(durationSeconds * 1000)
  : 10000;

function hr() {
  console.log("------------------------------------------------------------");
}

function prettyPayload(text) {
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    return text;
  }
}

function elapsedSince(startedAt) {
  return `${((Date.now() - startedAt) / 1000).toFixed(2)}s`;
}

console.log("SSE Demo");
hr();
console.log(`URL      : ${url}`);
console.log(`Duration : ${(durationMs / 1000).toFixed(1)}s`);
console.log("Pattern  : for await...of");
hr();

const startedAt = Date.now();
let messageCount = 0;

const sse = new Rong.SSE(url);

// Auto-close after duration
const timer = setTimeout(() => sse.close(), durationMs);

try {
  console.log(`[${elapsedSince(startedAt)}] connected`);

  for await (const event of sse) {
    messageCount += 1;

    console.log("");
    console.log(`[event ${messageCount}]`);
    console.log(`type   : ${event.type}`);
    console.log(`id     : ${event.id || "-"}`);
    console.log(`origin : ${event.origin || "-"}`);
    console.log("data   :");
    console.log(prettyPayload(event.data));
  }
} catch (e) {
  console.log(`[${elapsedSince(startedAt)}] error: ${e.message}`);
}

hr();
console.log(`Finished after ${elapsedSince(startedAt)}`);
console.log(`Messages received: ${messageCount}`);
