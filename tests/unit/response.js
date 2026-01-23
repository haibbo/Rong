describe("Response", () => {
  describe("constructor", () => {
    it("should create an empty response", () => {
      const response = new Response();
      expect(response.ok).toBe(true);
      expect(response.status).toBe(200);
      expect(response.statusText).toBe("");
      expect(response.headers instanceof Headers).toBe(true);
    });

    it("should create a response with body and init parameters", () => {
      const init = {
        status: 201,
        statusText: "Created",
        headers: { "Content-Type": "application/json" },
      };
      const body = JSON.stringify({ id: 1 });
      const response = new Response(body, init);

      expect(response.status).toBe(201);
      expect(response.statusText).toBe("Created");
      expect(response.headers.get("Content-Type")).toBe("application/json");
      expect(response.ok).toBe(true);
    });
  });

  describe("status validation", () => {
    it("should set ok to true for successful status codes", () => {
      const successCodes = [200, 201, 202, 203, 204, 206, 207, 299];
      successCodes.forEach((status) => {
        const response = new Response(null, { status });
        expect(response.ok).toBe(true);
      });
    });

    it("should set ok to false for error status codes", () => {
      const errorCodes = [400, 404, 500, 503];
      errorCodes.forEach((status) => {
        const response = new Response(null, { status });
        expect(response.ok).toBe(false);
      });
    });

    it("should throw TypeError for invalid status codes", () => {
      expect(() => new Response(null, { status: 99 })).toThrow(TypeError);
      expect(() => new Response(null, { status: 600 })).toThrow(TypeError);
    });
  });

  describe("body handling", () => {
    it("should handle different body types", async () => {
      const testCases = [
        {
          body: JSON.stringify({ test: "data" }),
          type: "application/json",
          check: async (response) => {
            const data = await response.json();
            expect(data.test).toBe("data");
          },
        },
        {
          body: "plain text",
          type: "text/plain",
          check: async (response) => {
            const text = await response.text();
            expect(text).toBe("plain text");
          },
        },
        {
          body: new ArrayBuffer(8),
          type: "application/octet-stream",
          check: async (response) => {
            const buffer = await response.arrayBuffer();
            expect(buffer instanceof ArrayBuffer).toBe(true);
            expect(buffer.byteLength).toBe(8);
          },
        },
        {
          body: "blob content",
          type: "text/plain",
          check: async (response) => {
            const blob = await response.blob();
            expect(blob instanceof Blob).toBe(true);
            expect(blob.type).toBe("text/plain");
            const text = await blob.text();
            expect(text).toBe("blob content");
          },
        },
      ];

      for (const testCase of testCases) {
        const response = new Response(testCase.body, {
          headers: { "Content-Type": testCase.type },
        });
        await testCase.check(response);
      }
    });

    it("should return the same body stream object across getter calls (buffered)", () => {
      const response = new Response("hello world");
      const b1 = response.body;
      const b2 = response.body;
      expect(b1 === b2).toBe(true);
    });

    it("should return the same body stream object across getter calls (JS body)", () => {
      const buf = new Uint8Array([1, 2, 3]);
      const response = new Response(buf);
      const b1 = response.body;
      const b2 = response.body;
      expect(b1 === b2).toBe(true);
    });

    it("should not allow multiple body reads", async () => {
      const response = new Response(JSON.stringify({ ok: true }), {
        headers: { "Content-Type": "application/json" },
      });

      const a = await response.json();
      expect(a.ok).toBe(true);

      // Second read must fail per spec.
      let threw = false;
      try {
        await response.text();
      } catch (err) {
        threw = true;
        expect(err instanceof TypeError).toBe(true);
        expect(/body used already/.test(String(err))).toBe(true);
      }
      if (!threw) {
        throw new Error("Expected second body read to fail");
      }
    });
  });

  describe("clone", () => {
    it("should create an identical copy of the response", async () => {
      const original = new Response("test data", {
        status: 200,
        headers: { "Content-Type": "text/plain" },
      });

      const clone = original.clone();

      expect(clone.status).toBe(original.status);
      expect(clone.headers.get("Content-Type")).toBe(
        original.headers.get("Content-Type"),
      );
      expect(clone).not.toBe(original);

      const originalText = await original.text();
      const cloneText = await clone.text();
      expect(cloneText).toBe(originalText);
    });
  });

  describe("error handling", () => {
    it("should create error responses", () => {
      const response = Response.error();
      expect(response.status).toBe(0);
      expect(response.statusText).toBe("");
      expect(response.ok).toBe(false);
    });

    it("should create redirect responses", () => {
      const url = "https://example.com/new-location";
      const response = Response.redirect(url, 301);
      expect(response.status).toBe(301);
      expect(response.headers.get("Location")).toBe(url);
    });

    it("should throw TypeError for invalid redirect status", () => {
      expect(() => Response.redirect("https://example.com", 200)).toThrow(
        TypeError,
      );
    });
  });
});
