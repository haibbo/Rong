describe("fetch", () => {
  it("should fetch IP from test server", async () => {
    const url = new URL("/ip", TEST_SERVER_URL);
    const response = await fetch(url);
    expect(response instanceof Response).toBe(true);
    expect(response.ok).toBe(true);
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("application/json");
    const data = await response.json();
    console.log(data);
    expect(typeof data.origin).toBe("string");
    expect(data.origin).toBe("127.0.0.1");
  });

  it("should send and receive custom headers", async () => {
    const url = new URL("/headers", TEST_SERVER_URL);
    const customHeaders = {
      "X-Custom-Header": "custom value",
      "X-Test-Header": "test value",
      "User-Agent": "RongJS Test Client",
    };

    const response = await fetch(url, {
      headers: customHeaders,
    });

    expect(response.ok).toBe(true);
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("application/json");

    const data = await response.json();
    console.log("Received headers:", data);

    // Verify our custom headers were received by the server
    for (const [key, value] of Object.entries(customHeaders)) {
      assert.equal(data[key.toLowerCase()], value);
    }
  });

  it("should handle gzipped response", async () => {
    const url = new URL("/gzip", TEST_SERVER_URL);
    const response = await fetch(url);
    expect(response instanceof Response).toBe(true);
    expect(response.ok).toBe(true);
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("application/json");
    expect(response.headers.get("content-encoding")).toBe("gzip");

    const data = await response.json();
    expect(data.gzipped).toBe(true);
    expect(typeof data.method).toBe("string");
    expect(data.method).toBe("GET");
  });

  it("should handle network errors", async () => {
    try {
      await fetch("https://invalid.example.com");
    } catch (error) {
      console.log(error);
      expect(error instanceof TypeError).toBe(true);
    }
  });
});

describe("Abort to fetch", () => {
  let controller;
  let signal;

  beforeEach(() => {
    controller = new AbortController();
    signal = controller.signal;
  });

  const waitForFetchStart = async (startFlag) => {
    while (!startFlag) {
      await new Promise((resolve) => setTimeout(resolve, 10));
    }
  };

  it("should abort fetch request", async () => {
    let fetchStarted = false;
    const fetchPromise = (async () => {
      fetchStarted = true;
      return await fetch(`${TEST_SERVER_URL}/delay`, { signal });
    })();

    await waitForFetchStart(fetchStarted);
    await new Promise((resolve) => setTimeout(resolve, 50));
    controller.abort();

    try {
      await fetchPromise;
      assert.fail("fetch should have been aborted");
    } catch (error) {
      assert.ok(error instanceof DOMException);
      assert.equal(error.name, "AbortError");
      console.log("##Got:", error.name);
    }
  });

  it("should abort during response body read", async () => {
    const response = await fetch(`${TEST_SERVER_URL}/large`, { signal });

    // Start reading the body first
    const readPromise = response.arrayBuffer();
    let abortCaught = false; // Add flag to track if we catch the abort

    // Wait just a tiny bit to ensure reading has started
    await new Promise((resolve) => setTimeout(resolve, 10));

    // Then abort
    controller.abort();

    try {
      await readPromise;
      console.log("##Body read completed without being aborted");
    } catch (error) {
      assert.ok(error instanceof DOMException);
      assert.equal(error.name, "AbortError");
      abortCaught = true; // Set flag when we catch the abort
      console.log("##Got: ", error.name);
    }

    // Verify that we actually caught the abort
    if (!abortCaught) {
      throw new Error("Body read was not aborted as expected");
    }
  });

  it("should abort with custom reason", async () => {
    const reason = new Error("Custom abort reason");
    assert.equal(signal.aborted, false);
    assert.equal(signal.reason, undefined);

    let fetchStarted = false;
    const fetchPromise = (async () => {
      fetchStarted = true;
      return await fetch(`${TEST_SERVER_URL}/delay`, { signal });
    })();

    await waitForFetchStart(fetchStarted);
    await new Promise((resolve) => setTimeout(resolve, 50));
    controller.abort(reason);

    assert.equal(signal.aborted, true);
    assert.equal(signal.reason, reason);

    try {
      await fetchPromise;
      assert.fail("fetch should have been aborted");
    } catch (error) {
      assert.equal(error, reason);
      console.log("##Got: ", error);
    }
  });

  it("should abort immediately if signal is already aborted", async () => {
    assert.equal(signal.aborted, false);
    assert.equal(signal.reason, undefined);

    controller.abort();

    assert.equal(signal.aborted, true);
    assert.ok(signal.reason instanceof DOMException);
    assert.equal(signal.reason.name, "AbortError");

    try {
      await fetch(`${TEST_SERVER_URL}/ip`, { signal });
      assert.fail("fetch should have been aborted");
    } catch (error) {
      console.log("##Got: ", error.name);
      assert.ok(error instanceof DOMException);
      assert.equal(error.name, "AbortError");
    }
  });
});
