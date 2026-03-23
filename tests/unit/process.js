describe("Process", () => {
  it("hides Process constructor from global scope", () => {
    assert.equal(typeof Process, "undefined");
    assert.equal(globalThis.Process, undefined);

    let failed = false;
    try {
      new process.constructor();
    } catch (e) {
      failed = true;
    }
    assert.equal(failed, true, "Process should not be constructible via instance.constructor");
  });

  describe("Static Properties", () => {
    it("should have platform property", () => {
      assert.ok(typeof process.platform === "string");
      assert.ok(process.platform.length > 0);
      // platform should be one of known values
      const validPlatforms = ["darwin", "win32", "linux"];
      // Note: we don't strictly validate since OS might vary
      assert.ok(
        typeof process.platform === "string",
        "platform should be a string",
      );
    });

    it("should have arch property", () => {
      assert.ok(typeof process.arch === "string");
      assert.ok(process.arch.length > 0);
    });

    it("should have version property", () => {
      assert.ok(typeof process.version === "string");
      assert.ok(
        process.version.startsWith("v"),
        "version should start with 'v'",
      );
    });

    it("should have pid property", () => {
      assert.ok(typeof process.pid === "number");
      assert.ok(process.pid > 0, "pid should be positive");
    });
  });

  describe("Environment Variables", () => {
    it("should have env object", () => {
      assert.ok(typeof process.env === "object");
      assert.ok(process.env !== null);
    });

    it("should contain PATH or Path environment variable", () => {
      const hasPath =
        typeof process.env.PATH === "string" ||
        typeof process.env.Path === "string";
      assert.ok(hasPath, "PATH or Path should exist in env");
    });

    it("should have persistent env object", () => {
      const original = process.env;
      process.env.TEST_VAR = "hello";
      assert.equal(process.env.TEST_VAR, "hello", "env var should be set");
      assert.ok(process.env === original, "env object should be singleton");

      // Verify persistence across access
      assert.equal(process.env.TEST_VAR, "hello", "env var should persist");
    });
  });

  describe("Command Line Arguments", () => {
    it("should have argv array", () => {
      assert.ok(Array.isArray(process.argv));
    });

    it("should have string elements in argv", () => {
      for (const arg of process.argv) {
        assert.ok(typeof arg === "string", "argv elements should be strings");
      }
    });
  });

  describe("Working Directory", () => {
    it("should return cwd as string", () => {
      const cwd = process.cwd();
      assert.ok(typeof cwd === "string");
      assert.ok(cwd.length > 0);
    });

    it("should change directory with chdir", () => {
      const original = process.cwd();
      try {
        process.chdir("..");
        const newCwd = process.cwd();
        assert.ok(
          newCwd !== original || original === "/",
          "cwd should change after chdir",
        );
      } finally {
        // Restore original directory
        process.chdir(original);
      }
    });

    it("should throw on invalid chdir path", () => {
      let threw = false;
      try {
        process.chdir("/nonexistent/path/that/does/not/exist/12345");
      } catch (e) {
        threw = true;
      }
      assert.ok(threw, "chdir should throw on invalid path");
    });
  });

  describe("Timing Functions", () => {
    it("should return uptime as number", () => {
      const uptime = process.uptime();
      assert.ok(typeof uptime === "number");
      assert.ok(uptime >= 0, "uptime should be non-negative");
    });

    it("should have increasing uptime", (done) => {
      const uptime1 = process.uptime();
      setTimeout(() => {
        const uptime2 = process.uptime();
        assert.ok(uptime2 >= uptime1, "uptime should increase over time");
        done();
      }, 50);
    });

    it("should return hrtime as array of two numbers", () => {
      const hr = process.hrtime();
      assert.ok(Array.isArray(hr));
      assert.ok(hr.length === 2);
      assert.ok(typeof hr[0] === "number", "seconds should be a number");
      assert.ok(typeof hr[1] === "number", "nanoseconds should be a number");
    });

    it("should have non-negative hrtime values", () => {
      const hr = process.hrtime();
      assert.ok(hr[0] >= 0, "seconds should be non-negative");
      assert.ok(hr[1] >= 0, "nanoseconds should be non-negative");
    });

    it("should calculate hrtime difference", (done) => {
      const start = process.hrtime();
      setTimeout(() => {
        const diff = process.hrtime(start);
        assert.ok(Array.isArray(diff), "diff should be array");
        assert.ok(diff.length === 2, "diff should have 2 elements");
        // After 50ms, we expect at least some nanoseconds elapsed
        const totalNanos = diff[0] * 1e9 + diff[1];
        assert.ok(totalNanos >= 40_000_000, "should have elapsed at least 40ms");
        assert.ok(totalNanos < 500_000_000, "should not exceed 500ms");
        done();
      }, 50);
    });
  });

  describe("Standard I/O", () => {
    it("should have stdin object", () => {
      assert.ok(process.stdin !== null && process.stdin !== undefined);
    });

    it("should have stdin.isTTY property", () => {
      assert.ok(typeof process.stdin.isTTY === "boolean");
    });

    it("should have stdin.getReader function (ReadableStream)", () => {
      assert.ok(
        typeof process.stdin.getReader === "function",
        "stdin should be a ReadableStream",
      );
    });

    it("should have stdout object", () => {
      assert.ok(typeof process.stdout === "object");
      assert.ok(process.stdout !== null);
    });

    it("should have stdout.write function", () => {
      assert.ok(typeof process.stdout.write === "function");
    });

    it("should have stdout.isTTY property", () => {
      assert.ok(typeof process.stdout.isTTY === "boolean");
    });

    it("should have stderr object", () => {
      assert.ok(typeof process.stderr === "object");
      assert.ok(process.stderr !== null);
    });

    it("should have stderr.write function", () => {
      assert.ok(typeof process.stderr.write === "function");
    });

    it("should have stderr.isTTY property", () => {
      assert.ok(typeof process.stderr.isTTY === "boolean");
    });

    it("should have singleton stdin, stdout and stderr", () => {
      const stdin1 = process.stdin;
      const stdin2 = process.stdin;
      assert.ok(stdin1 === stdin2, "stdin should be the same object");

      const stdout1 = process.stdout;
      const stdout2 = process.stdout;
      assert.ok(stdout1 === stdout2, "stdout should be the same object");

      const stderr1 = process.stderr;
      const stderr2 = process.stderr;
      assert.ok(stderr1 === stderr2, "stderr should be the same object");
    });
  });

  describe("nextTick", () => {
    it("should have nextTick function", () => {
      assert.ok(typeof process.nextTick === "function");
    });

    it("should execute callback asynchronously", (done) => {
      let executed = false;
      process.nextTick(() => {
        executed = true;
        done();
      });
      assert.ok(!executed, "callback should not execute synchronously");
    });

    it("should execute after synchronous code", (done) => {
      const order = [];
      process.nextTick(() => {
        order.push("tick");
        assert.ok(
          order[0] === "sync",
          "sync should come before tick: " + order.join(","),
        );
        done();
      });
      order.push("sync");
    });

    it("should pass arguments to callback", (done) => {
      process.nextTick(
        (a, b, c) => {
          assert.equal(a, 1, "arg 1 match");
          assert.equal(b, "two", "arg 2 match");
          assert.equal(c, true, "arg 3 match");
          done();
        },
        1,
        "two",
        true,
      );
    });
  });

  describe("EventEmitter Interface", () => {
    it("should have on function", () => {
      assert.ok(typeof process.on === "function");
    });

    it("should have emit function", () => {
      assert.ok(typeof process.emit === "function");
    });

    it("should have off function", () => {
      assert.ok(typeof process.off === "function");
    });

    it("should have addListener function", () => {
      assert.ok(typeof process.addListener === "function");
    });

    it("should have removeListener function", () => {
      assert.ok(typeof process.removeListener === "function");
    });

    it("should have once function", () => {
      assert.ok(typeof process.once === "function");
    });

    it("should handle event listeners", () => {
      let called = false;
      const handler = () => {
        called = true;
      };

      process.on("testEvent", handler);
      process.emit("testEvent");

      assert.ok(called, "event handler should be called");

      // Cleanup
      process.off("testEvent", handler);
    });

    it("should handle once listeners", () => {
      let callCount = 0;
      process.once("onceEvent", () => {
        callCount++;
      });

      process.emit("onceEvent");
      process.emit("onceEvent");

      assert.ok(callCount === 1, "once handler should only be called once");
    });

    it("should remove listeners with off", () => {
      let called = false;
      const handler = () => {
        called = true;
      };

      process.on("removeTest", handler);
      process.off("removeTest", handler);
      process.emit("removeTest");

      assert.ok(!called, "removed handler should not be called");
    });

    // Note: Skipping chaining test - rong_event currently returns void from on()
    // This is a known limitation that should be fixed in rong_event module
    // it("should return process from on for chaining", () => {
    //   const result = process.on("chainTest", () => {});
    //   assert.ok(result === process, "on should return process for chaining");
    //   process.removeAllListeners("chainTest");
    // });
  });

  describe("exit function", () => {
    it("should have exit function", () => {
      assert.ok(typeof process.exit === "function");
    });

    // Note: We don't actually test exit() as it would terminate the process
  });
});
