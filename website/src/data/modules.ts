// The 20 built-in modules (workspace members under modules/).
// Module API pages are generated from the repo's docs/api/*.md at build
// time — see src/content.config.ts. rong_cron has no docs/api entry yet and
// falls back to modules/rong_cron/README.md.
export const MODULES: [name: string, desc: string][] = [
  ["rong_timer", "setTimeout, setInterval, async timers"],
  ["rong_http", "HTTP client/server, fetch API"],
  ["rong_fs", "File system operations"],
  ["rong_console", "Console logging & debugging"],
  ["rong_url", "URL parsing & manipulation"],
  ["rong_buffer", "Binary data handling"],
  ["rong_event", "Event emitter & handling"],
  ["rong_abort", "AbortController & signals"],
  ["rong_encoding", "Text encoding / decoding"],
  ["rong_assert", "Assertion utilities"],
  ["rong_exception", "Exception handling"],
  ["rong_storage", "Storage APIs"],
  ["rong_stream", "Stream APIs"],
  ["rong_compression", "Compression & decompression"],
  ["rong_command", "Subprocess & shell execution"],
  ["rong_worker", "JavaScript worker threads"],
  ["rong_cron", "Cron parsing & scheduled jobs"],
  ["rong_redis", "Redis client APIs"],
  ["rong_sqlite", "SQLite APIs"],
  ["rong_s3", "S3-compatible object storage"],
];
