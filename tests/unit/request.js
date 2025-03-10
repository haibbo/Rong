describe("Request", () => {
  describe("constructor", () => {
    it("should create a request with minimum required parameters", () => {
      const request = new Request("https://example.com");
      expect(request.url).toBe("https://example.com/");
      expect(request.method).toBe("GET");
      expect(request.headers instanceof Headers).toBe(true);
    });

    it("should create a request with all parameters", () => {
      const init = {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ key: "value" }),
        redirect: "follow",
      };
      const request = new Request("https://api.example.com", init);

      expect(request.url).toBe("https://api.example.com/");
      expect(request.method).toBe("POST");
      expect(request.headers.get("Content-Type")).toBe("application/json");
      expect(request.cache).toBe("no-cache");
      expect(request.redirect).toBe("follow");
    });

    it("should create a request from URL object", () => {
      const url = new URL("https://example.com/path?query=value");
      const request = new Request(url);
      expect(request.url).toBe("https://example.com/path?query=value");
      expect(request.method).toBe("GET");
      expect(request.headers instanceof Headers).toBe(true);
    });

    it("should throw TypeError for invalid URL", () => {
      expect(() => new Request("not-a-url")).toThrow(TypeError);
    });
  });

  describe("method validation", () => {
    it("should allow standard HTTP methods", () => {
      const methods = [
        "GET",
        "POST",
        "PUT",
        "DELETE",
        "HEAD",
        "OPTIONS",
        "PATCH",
      ];
      methods.forEach((method) => {
        const request = new Request("https://example.com", { method });
        expect(request.method).toBe(method);
      });
    });

    it("should throw TypeError for invalid HTTP methods", () => {
      expect(
        () => new Request("https://example.com", { method: "INVALID" }),
      ).toThrow(TypeError);
    });
  });

  describe("body handling", () => {
    it("should not allow body for GET/HEAD requests", () => {
      expect(
        () =>
          new Request("https://example.com", {
            method: "GET",
            body: "test",
          }),
      ).toThrow();

      expect(
        () =>
          new Request("https://example.com", {
            method: "HEAD",
            body: "test",
          }),
      ).toThrow();
    });

    it("should allow body for POST requests", () => {
      const bodies = [
        JSON.stringify({ test: "data" }),
        new URLSearchParams("key=value"),
        "plain text",
      ];

      bodies.forEach((body) => {
        const request = new Request("https://example.com", {
          method: "POST",
          body,
        });
        expect(request.method).toBe("POST");
      });
    });
  });

  describe("clone", () => {
    it("should create an identical copy of the request", () => {
      const original = new Request("https://example.com", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ test: "data" }),
      });

      const clone = original.clone();

      expect(clone.url).toBe(original.url);
      expect(clone.method).toBe(original.method);
      expect(clone.headers.get("Content-Type")).toBe(
        original.headers.get("Content-Type"),
      );
      expect(clone).not.toBe(original);
    });
  });
});
