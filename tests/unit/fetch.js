describe("fetch", () => {
  it("should fetch IP from httpbin", async () => {
    const url = new URL("https://httpbin.org/ip");
    const response = await fetch(url);
    expect(response instanceof Response).toBe(true);
    expect(response.ok).toBe(true);
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("application/json");
    const data = await response.json();
    console.log(data);
    expect(typeof data.origin).toBe("string");

    // Validate IP address format without using regex
    const parts = data.origin.split(".");
    expect(parts.length).toBe(4);
    parts.forEach((part) => {
      const num = parseInt(part, 10);
      expect(num >= 0 && num <= 255).toBe(true);
    });
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
