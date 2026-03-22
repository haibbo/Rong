// S3 namespace prefix tests.
// The Rust harness injects a pre-configured `s3` global with namespace prefix "app1/".
// JS never calls `new S3Client` — it uses the injected instance directly.

describe("S3 namespace prefix", () => {
  const KEY = `ns-test-${Date.now()}.txt`;
  const CONTENT = "namespaced content";

  beforeEach(async () => {
    await s3.write(KEY, CONTENT);
  });

  afterEach(async () => {
    if (await s3.exists(KEY)) {
      await s3.delete(KEY);
    }
  });

  it("write and read through namespaced client", async () => {
    const n = await s3.write(KEY, CONTENT);
    assert.equal(n, CONTENT.length);

    const file = s3.file(KEY);
    const text = await file.text();
    assert.equal(text, CONTENT);
  });

  it("file() keeps the namespace transparent", () => {
    const file = s3.file(KEY);
    assert.equal(file.name, KEY);
  });

  it("exists returns true for namespaced key", async () => {
    assert.equal(await s3.exists(KEY), true);
  });

  it("size returns correct byte count", async () => {
    assert.equal(await s3.size(KEY), CONTENT.length);
  });

  it("stat returns metadata", async () => {
    const st = await s3.stat(KEY);
    assert(st.size > 0, "stat.size > 0");
  });

  it("list with prefix returns namespaced keys (prefix stripped)", async () => {
    const result = await s3.list({ prefix: "ns-test-" });
    assert(result.contents.length > 0, "found objects");
    // Keys should NOT contain the namespace prefix — it's transparent
    const found = result.contents.find((o) => o.key === KEY);
    assert(found, "found key without namespace prefix");
  });

  it("file operations are isolated from non-prefixed keys", async () => {
    // Write directly via the global S3Client (no namespace) to verify isolation
    const raw = new S3Client({
      accessKeyId: TEST_S3_ACCESS_KEY,
      secretAccessKey: TEST_S3_SECRET_KEY,
      bucket: TEST_S3_BUCKET,
      endpoint: TEST_S3_ENDPOINT,
    });

    // The namespaced client wrote to "app1/<KEY>", so a raw client
    // should NOT find it under just "<KEY>"
    const rawExists = await raw.exists(KEY);
    assert.equal(rawExists, false, "raw client should not see namespaced key");

    // But it should exist under the full prefixed path
    const prefixedExists = await raw.exists("app1/" + KEY);
    assert.equal(prefixedExists, true, "raw client sees full prefixed key");
  });

  it("delete removes namespaced key", async () => {
    await s3.delete(KEY);
    assert.equal(await s3.exists(KEY), false);
  });
});
