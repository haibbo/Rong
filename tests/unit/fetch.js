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
