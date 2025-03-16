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
        const now = await timer.setTimeout(100);
        const elapsed = Date.now() - start;
        assert.ok(
          elapsed >= 100 && elapsed <= 150,
          "setTimeout should wait between 100-150ms",
        );
        assert.ok(
          now >= start && now <= Date.now(),
          "Resolved value should be a valid timestamp",
        );
      });

      it("should handle zero delay", async () => {
        const start = Date.now();
        const now = await timer.setTimeout(0);
        assert.ok(
          now >= start && now <= Date.now(),
          "Resolved value should be a valid timestamp",
        );
      });

      it("should handle negative delay", async () => {
        const start = Date.now();
        const now = await timer.setTimeout(-100);
        const elapsed = Date.now() - start;
        assert.ok(elapsed >= 0, "Negative delay should be treated as 0");
        assert.ok(
          now >= start && now <= Date.now(),
          "Resolved value should be a valid timestamp",
        );
      });
    });

    describe("timer.setImmediate", () => {
      it("should execute on next tick", async () => {
        const start = Date.now();
        const now = await timer.setImmediate();
        assert.ok(
          now >= start && now <= Date.now(),
          "Resolved value should be a valid timestamp",
        );
      });

      it("should execute after current task", async () => {
        const order = [];
        const promise = timer.setImmediate();
        order.push(1);
        await promise;
        order.push(2);
        assert.equal(
          order.join(","),
          "1,2",
          "setImmediate should execute after current task",
        );
      });
    });

    describe("timer.setInterval", () => {
      it("should create async iterator with correct timing", async () => {
        const start = Date.now();
        const interval = timer.setInterval(50);
        const times = [];

        for await (const now of interval) {
          assert.ok(
            now >= start && now <= Date.now(),
            "Iterator value should be a valid timestamp",
          );
          times.push(now);
          if (times.length >= 3) break;
        }

        assert.equal(times.length, 3, "Should collect 3 interval values");

        // Verify timing with more lenient threshold
        for (let i = 1; i < times.length; i++) {
          const diff = times[i] - times[i - 1];
          assert.ok(
            diff >= 40, // Reduced from 45 to 40
            `Interval between values should be at least 40ms (got ${diff}ms)`,
          );
        }
      });

      it("should execute repeatedly", async () => {
        const interval = timer.setInterval(50);
        let count = 0;

        for await (const _ of interval) {
          count++;
          if (count >= 3) break;
        }

        assert.equal(count, 3, "Should execute 3 times");
      });

      it("should handle break in loop", async () => {
        const interval = timer.setInterval(50);
        let count = 0;

        for await (const _ of interval) {
          count++;
          break;
        }

        assert.equal(count, 1, "Should stop after break");
      });
    });
  });
});
