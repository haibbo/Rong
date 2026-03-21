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
console.log("Handlers : onopen / onmessage / onerror");
hr();

const startedAt = Date.now();
let messageCount = 0;
let closed = false;

const evtSource = new EventSource(url);

function closeSource() {
  if (closed) return;
  closed = true;
  evtSource.close();
}

await new Promise((resolve, reject) => {
  evtSource.onopen = function() {
    console.log(`[${elapsedSince(startedAt)}] connected`);
  };

  evtSource.onmessage = function(event) {
    messageCount += 1;

    console.log("");
    console.log(`[event ${messageCount}]`);
    console.log(`type   : ${event.type}`);
    console.log(`id     : ${event.lastEventId || "-"}`);
    console.log(`origin : ${event.origin || "-"}`);
    console.log("data   :");
    console.log(prettyPayload(event.data));
  };

  evtSource.onerror = function(event) {
    closeSource();
    reject(new Error(event.message || "SSE error"));
  };

  setTimeout(() => {
    closeSource();
    resolve();
  }, durationMs);
});

hr();
console.log(`Finished after ${elapsedSince(startedAt)}`);
console.log(`Messages received: ${messageCount}`);
