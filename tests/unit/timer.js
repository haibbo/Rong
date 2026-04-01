describe("Timer", () => {
  describe("setTimeout", () => {
    it("should execute callback after delay", (done) => {
      const start = Date.now();
      const timeoutId = setTimeout(() => {
        const elapsed = Date.now() - start;
        assert.ok(
          elapsed >= 100 && elapsed <= 180,
          "setTimeout should wait between 100-180ms",
        );
        done();
      }, 100);
      assert.ok(
        typeof timeoutId === "number",
        "setTimeout should return a number id",
      );
    });

    it("should handle clearing timeout", (done) => {
      let called = false;
      const timeoutId = setTimeout(() => {
        called = true;
      }, 50);

      clearTimeout(timeoutId);

      setTimeout(() => {
        assert.ok(!called, "Callback should not be called after clearTimeout");
        done();
      }, 100);
    });
  });

  describe("setInterval", () => {
    it("should execute callback repeatedly", (done) => {
      const results = [];
      const intervalId = setInterval(() => {
        results.push(Date.now());
        if (results.length >= 3) {
          clearInterval(intervalId);

          for (let i = 1; i < results.length; i++) {
            const diff = results[i] - results[i - 1];
            assert.ok(
              diff >= 45,
              "Interval between values should be at least 45ms",
            );
          }
          done();
        }
      }, 50);

      assert.ok(
        typeof intervalId === "number",
        "setInterval should return a number id",
      );
    });

    it("should handle clearing interval", (done) => {
      let count = 0;
      const intervalId = setInterval(() => {
        count++;
      }, 50);

      setTimeout(() => {
        clearInterval(intervalId);
        const currentCount = count;

        setTimeout(() => {
          assert.equal(
            count,
            currentCount,
            "Count should not increase after clearInterval",
          );
          done();
        }, 100);
      }, 125);
    });
  });

  describe("edge cases", () => {
    it("should handle clearing non-existent timers", () => {
      clearTimeout(999999);
      clearInterval(999999);
    });

    it("should not expose the removed timers namespace", () => {
      assert.equal(globalThis.timers, undefined);
    });
  });
});
