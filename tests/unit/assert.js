describe("assert module", () => {
  describe("assert.ok()", () => {
    it("should pass for truthy values", () => {
      assert.ok(true);
      assert.ok(1);
      assert.ok("test");
      assert.ok({});
      assert.ok([]);
    });

    it("should throw for falsy values", () => {
      expect(() => assert.ok(false)).toThrow();
      expect(() => assert.ok(0)).toThrow();
      expect(() => assert.ok("")).toThrow();
      expect(() => assert.ok(null)).toThrow();
      expect(() => assert.ok(undefined)).toThrow();
    });

    it("should include custom message in error", () => {
      const message = "Custom error message";
      try {
        assert.ok(false, message);
      } catch (e) {
        expect(e.message).toContain(message);
      }
    });
  });

  describe("assert.equal()", () => {
    it("should pass for equal values", () => {
      assert.equal(1, 1);
      assert.equal("test", "test");
      assert.equal(null, null);
      assert.equal(undefined, undefined);
    });

    it("should throw for unequal values", () => {
      expect(() => assert.equal(1, 2)).toThrow();
      expect(() => assert.equal("a", "b")).toThrow();
    });

    it("should use loose equality", () => {
      assert.equal(1, "1");
      assert.equal(0, false);
      assert.equal("", false);
    });

    it("should include custom message in error", () => {
      const message = "Custom equality message";
      try {
        assert.equal(1, 2, message);
      } catch (e) {
        expect(e.message).toContain(message);
      }
    });
  });

  describe("assert.fail()", () => {
    it("should always throw", () => {
      expect(() => assert.fail()).toThrow();
    });

    it("should include custom message in error", () => {
      const message = "Custom fail message";
      try {
        assert.fail(message);
      } catch (e) {
        expect(e.message).toContain(message);
      }
    });
  });

  describe("assert.doesNotThrow()", () => {
    it("should pass for functions that don't throw", () => {
      assert.doesNotThrow(() => {
        // This function doesn't throw
        return true;
      });
    });

    it("should throw for non-function arguments", () => {
      expect(() => assert.doesNotThrow("not a function")).toThrow();
      expect(() => assert.doesNotThrow(123)).toThrow();
      expect(() => assert.doesNotThrow({})).toThrow();
    });

    it("should include custom message in error", () => {
      const message = "Custom doesNotThrow message";
      try {
        assert.doesNotThrow(() => {
          throw new Error("Test error");
        }, message);
      } catch (e) {
        expect(e.message).toContain(message);
      }
    });
  });

  describe("assert.fail()", () => {
    it("should always throw", () => {
      expect(() => assert.fail()).toThrow();
    });

    it("should include custom message in error", () => {
      const message = "Custom fail message";
      try {
        assert.fail(message);
      } catch (e) {
        expect(e.message).toContain(message);
      }
    });

    it("should throw with default message when no args", () => {
      try {
        assert.fail();
      } catch (e) {
        expect(e.message).toBe("Failed");
      }
    });

    it("should throw with provided error object", () => {
      const error = new Error("Custom error");
      try {
        assert.fail(error);
      } catch (e) {
        expect(e).toBe(error);
      }
    });
  });
});
