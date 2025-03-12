describe("Timer", () => {
  describe("Synchronous APIs", () => {
    describe("setTimeout", () => {
      it("should execute callback after delay", (done) => {
        const start = Date.now();
        const timeoutId = setTimeout(() => {
          const elapsed = Date.now() - start;
          assert.ok(
            elapsed >= 100 && elapsed <= 150,
            "setTimeout should wait between 100-150ms",
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
          assert.ok(
            !called,
            "Callback should not be called after clearTimeout",
          );
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

            // Verify timing
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
    });
  });

  describe("Asynchronous APIs", () => {
    describe("timer.setTimeout", () => {
      it("should resolve after delay", async () => {
        const start = Date.now();
        await timer.setTimeout(() => {}, 100);
        const elapsed = Date.now() - start;
        assert.ok(
          elapsed >= 100 && elapsed <= 150,
          "setTimeout should wait between 100-150ms",
        );
      });

      it("should execute callback with result", async () => {
        const result = await timer.setTimeout(() => "test result", 50);
        assert.equal(result, "test result", "Should return callback result");
      });

      it("should handle zero delay", async () => {
        const result = await timer.setTimeout(() => "immediate", 0);
        assert.equal(result, "immediate", "Should execute immediately");
      });

      it("should handle negative delay", async () => {
        const start = Date.now();
        await timer.setTimeout(() => {}, -100);
        const elapsed = Date.now() - start;
        assert.ok(elapsed >= 0, "Negative delay should be treated as 0");
      });

      it("should handle errors in callback", async () => {
        try {
          await timer.setTimeout(() => {
            throw new Error("Test error");
          }, 50);
          assert.fail("Should have thrown an error");
        } catch (error) {
          assert.ok(error instanceof Error, "Should catch callback error");
          assert.equal(
            error.message,
            "Test error",
            "Should preserve error message",
          );
        }
      });
    });

    describe("timer.setImmediate", () => {
      it("should execute after current task", async () => {
        let order = [];
        order.push(1);
        await timer.setImmediate(() => {
          order.push(3);
        });
        order.push(2);
        assert.equal(
          order.join(","),
          "1,2,3",
          "setImmediate should execute after current task",
        );
      });

      it("should return callback result", async () => {
        const result = await timer.setImmediate(() => "immediate result");
        assert.equal(
          result,
          "immediate result",
          "Should return callback result",
        );
      });

      it("should handle errors in callback", async () => {
        try {
          await timer.setImmediate(() => {
            throw new Error("Test error");
          });
          assert.fail("Should have thrown an error");
        } catch (error) {
          assert.ok(error instanceof Error, "Should catch callback error");
          assert.equal(
            error.message,
            "Test error",
            "Should preserve error message",
          );
        }
      });
    });

    describe("timer.setInterval", () => {
      it("should create async iterator with correct timing", async () => {
        const interval = timer.setInterval(() => Date.now(), 50);
        const values = [];

        for await (const value of interval) {
          values.push(value);
          console.log("interval:", values.length);
          if (values.length >= 3) break;
        }

        assert.equal(values.length, 3, "Should collect 3 interval values");

        // Verify timing
        for (let i = 1; i < values.length; i++) {
          const diff = values[i] - values[i - 1];
          assert.ok(
            diff >= 45,
            "Interval between values should be at least 45ms",
          );
        }
      });

      it("should handle callback results", async () => {
        const interval = timer.setInterval(() => "tick", 50);
        const values = [];

        for await (const value of interval) {
          values.push(value);
          if (values.length >= 3) break;
        }

        assert.equal(
          values.join(","),
          "tick,tick,tick",
          "Should receive correct values",
        );
      });

      it("should handle errors in callback", async () => {
        const interval = timer.setInterval(() => {
          throw new Error("Test error");
        }, 50);

        try {
          for await (const _ of interval) {
            break;
          }
          assert.fail("Should have thrown an error");
        } catch (error) {
          assert.ok(error instanceof Error, "Should catch callback error");
          assert.equal(
            error.message,
            "Test error",
            "Should preserve error message",
          );
        }
      });
    });
  });
});
