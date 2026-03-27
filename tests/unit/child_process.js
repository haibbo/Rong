describe("Child Process", () => {
  it("hides internal child process classes from global scope", async () => {
    assert.equal(typeof ChildProcess, "undefined");
    assert.equal(typeof ExecResult, "undefined");

    const child = child_process.spawn("echo", ["hello"]);
    let childFailed = false;
    try {
      new child.constructor();
    } catch (e) {
      childFailed = true;
    }
    assert.equal(childFailed, true, "ChildProcess should not be constructible via instance.constructor");
    await child.wait();
  });

  describe("exec", () => {
    it("should execute a shell command and return stdout", async () => {
      const result = await child_process.exec("echo hello");
      assert.ok(
        result.stdout.includes("hello"),
        "stdout should contain 'hello'",
      );
      assert.equal(result.code, 0, "exit code should be 0");
    });

    it("should capture stderr", async () => {
      const result = await child_process.exec("echo error >&2");
      assert.ok(
        result.stderr.includes("error"),
        "stderr should contain 'error'",
      );
    });

    it("should return non-zero exit code for failing command", async () => {
      const result = await child_process.exec("exit 1");
      assert.equal(result.code, 1, "exit code should be 1");
    });

    it("should support cwd option", async () => {
      const result = await child_process.exec("pwd", { cwd: "/tmp" });
      assert.ok(
        result.stdout.includes("/tmp") ||
          result.stdout.includes("/private/tmp"),
        "should execute in /tmp directory",
      );
    });

    it("should support env option", async () => {
      const result = await child_process.exec("echo $TEST_CHILD_VAR", {
        env: { TEST_CHILD_VAR: "hello_from_env", PATH: process.env.PATH },
      });
      assert.ok(
        result.stdout.includes("hello_from_env"),
        "should use custom env variable",
      );
    });

    it("should inherit process.env modifications", async () => {
      // Modify process.env
      process.env.CHILD_PROCESS_TEST_VAR = "modified_value_123";

      // Spawn child without explicit env option - should inherit modified process.env
      const result = await child_process.exec(
        "echo $CHILD_PROCESS_TEST_VAR",
      );
      assert.ok(
        result.stdout.includes("modified_value_123"),
        "child should receive modified process.env",
      );

      // Clean up
      delete process.env.CHILD_PROCESS_TEST_VAR;
    });

    it("should handle command not found", async () => {
      const result = await child_process.exec(
        "nonexistent_cmd_xyz_12345 2>/dev/null || echo 'failed'",
      );
      // Either non-zero exit or contains 'failed'
      assert.ok(
        result.code !== 0 || result.stdout.includes("failed"),
        "should indicate failure for nonexistent command",
      );
    });

    it("should support timeout option", async () => {
      const start = Date.now();
      try {
        await child_process.exec("sleep 10", { timeout: 100 });
        assert.fail("should have timed out");
      } catch (e) {
        const elapsed = Date.now() - start;
        assert.ok(elapsed < 1000, "should timeout quickly");
        assert.ok(
          e.message.includes("timed out") || e.code === "ETIMEDOUT",
          "should have timeout error",
        );
      }
    });

    it("should complete before timeout", async () => {
      const result = await child_process.exec("echo fast", { timeout: 5000 });
      assert.ok(result.stdout.includes("fast"), "should complete successfully");
    });
  });

  describe("execFile", () => {
    it("should execute a file directly", async () => {
      const result = await child_process.execFile("/bin/echo", [
        "hello",
        "world",
      ]);
      assert.ok(
        result.stdout.includes("hello world"),
        "stdout should contain 'hello world'",
      );
      assert.equal(result.code, 0, "exit code should be 0");
    });

    it("should pass arguments correctly", async () => {
      const result = await child_process.execFile("/bin/echo", [
        "-n",
        "test",
      ]);
      assert.ok(result.stdout.includes("test"), "stdout should contain 'test'");
    });

    it("should support cwd option", async () => {
      const result = await child_process.execFile("/bin/pwd", [], {
        cwd: "/tmp",
      });
      assert.ok(
        result.stdout.includes("/tmp") ||
          result.stdout.includes("/private/tmp"),
        "should execute in /tmp directory",
      );
    });

    it("should handle special characters in arguments", async () => {
      const result = await child_process.execFile("/bin/echo", [
        "hello world",
        "foo'bar",
        'baz"qux',
      ]);
      assert.ok(
        result.stdout.includes("hello world"),
        "should handle spaces in args",
      );
      assert.ok(
        result.stdout.includes("foo'bar"),
        "should handle single quotes in args",
      );
      assert.ok(
        result.stdout.includes('baz"qux'),
        "should handle double quotes in args",
      );
    });

    it("should support timeout option", async () => {
      const start = Date.now();
      try {
        await child_process.execFile("/bin/sleep", ["10"], { timeout: 100 });
        assert.fail("should have timed out");
      } catch (e) {
        const elapsed = Date.now() - start;
        assert.ok(elapsed < 1000, "should timeout quickly");
        assert.ok(
          e.message.includes("timed out") || e.code === "ETIMEDOUT",
          "should have timeout error",
        );
      }
    });
  });

  describe("spawn", () => {
    it("should spawn a process and return ChildProcess object", async () => {
      const child = child_process.spawn("echo", ["hello"]);
      assert.ok(child !== null, "should return a ChildProcess object");
      assert.ok(typeof child.pid === "number", "should have pid property");
      await child.wait();
    });

    it("should have stdin, stdout and stderr streams", async () => {
      const child = child_process.spawn("cat", []);
      assert.ok(child.stdin !== null, "should have stdin");
      assert.ok(child.stdout !== null, "should have stdout");
      assert.ok(child.stderr !== null, "should have stderr");
      // Check stdin is WritableStream
      assert.ok(
        typeof child.stdin.getWriter === "function",
        "stdin should be a WritableStream",
      );
      // Check stdout/stderr are ReadableStreams
      assert.ok(
        typeof child.stdout.getReader === "function",
        "stdout should be a ReadableStream",
      );
      assert.ok(
        typeof child.stderr.getReader === "function",
        "stderr should be a ReadableStream",
      );
      // Clean up
      child.kill();
      await child.wait();
    });

    it("should write to stdin and read from stdout", async () => {
      const child = child_process.spawn("cat", []);

      // Write to stdin
      const writer = child.stdin.getWriter();
      const encoder = new TextEncoder();
      await writer.write(encoder.encode("hello from stdin"));
      await writer.close();

      // Read from stdout
      const reader = child.stdout.getReader();
      let output = "";
      const decoder = new TextDecoder();

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        output += decoder.decode(value);
      }

      assert.ok(
        output.includes("hello from stdin"),
        "should read what was written to stdin",
      );
    });

    it("should read stdout data via stream", async () => {
      const child = child_process.spawn("echo", ["hello streaming"]);
      const reader = child.stdout.getReader();
      let output = "";
      const decoder = new TextDecoder();

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        output += decoder.decode(value);
      }

      assert.ok(
        output.includes("hello streaming"),
        "should read 'hello streaming' from stdout",
      );
    });

    it("should support shell option", async () => {
      const child = child_process.spawn("echo $HOME", [], { shell: true });
      const reader = child.stdout.getReader();
      let output = "";
      const decoder = new TextDecoder();

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        output += decoder.decode(value);
      }

      assert.ok(output.length > 0, "should receive HOME directory");
    });

    it("should support cwd option", async () => {
      const child = child_process.spawn("pwd", [], { cwd: "/tmp" });
      const reader = child.stdout.getReader();
      let output = "";
      const decoder = new TextDecoder();

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        output += decoder.decode(value);
      }

      assert.ok(
        output.includes("/tmp") || output.includes("/private/tmp"),
        "should execute in /tmp directory",
      );
    });

    it("should get exitCode via wait()", async () => {
      const child = child_process.spawn("echo", ["test"]);
      const code = await child.wait();
      assert.equal(code, 0, "exitCode should be 0");
    });

    it("should capture non-zero exit code via wait()", async () => {
      const child = child_process.spawn("exit", ["42"], { shell: true });
      const code = await child.wait();
      assert.equal(code, 42, "exitCode should be 42");
    });

    it("should emit exit event", async () => {
      const child = child_process.spawn("echo", ["test"]);
      const code = await new Promise((resolve) => {
        child.once("exit", resolve);
      });
      assert.equal(code, 0, "exit event should provide exit code");
    });
  });

  describe("ChildProcess properties", () => {
    it("should have pid after spawn", async () => {
      const child = child_process.spawn("echo", ["test"]);
      assert.ok(typeof child.pid === "number", "pid should be a number");
      assert.ok(child.pid > 0, "pid should be positive");
      await child.wait();
    });

    it("should have kill method", async () => {
      const child = child_process.spawn("sleep", ["10"]);
      assert.ok(typeof child.kill === "function", "should have kill method");
      // Kill the sleep process with default SIGTERM
      const result = child.kill();
      assert.equal(result, true, "kill should return true for running process");
      await child.wait();
    });

    it("should support kill with signal name", async () => {
      const child = child_process.spawn("sleep", ["10"]);
      const result = child.kill("SIGKILL");
      assert.equal(result, true, "kill with SIGKILL should return true");
      await child.wait();
    });

    it("should support kill with signal number", async () => {
      const child = child_process.spawn("sleep", ["10"]);
      const result = child.kill("9"); // SIGKILL
      assert.equal(result, true, "kill with signal number should return true");
      await child.wait();
    });

    it("should have wait method", async () => {
      const child = child_process.spawn("echo", ["test"]);
      assert.ok(typeof child.wait === "function", "should have wait method");
      await child.wait();
    });
  });

  describe("EventEmitter interface", () => {
    it("should have on method", async () => {
      const child = child_process.spawn("echo", ["test"]);
      assert.ok(typeof child.on === "function", "should have on method");
      await child.wait();
    });

    it("should have once method", async () => {
      const child = child_process.spawn("echo", ["test"]);
      assert.ok(typeof child.once === "function", "should have once method");
      await child.wait();
    });

    it("should have off method", async () => {
      const child = child_process.spawn("echo", ["test"]);
      assert.ok(typeof child.off === "function", "should have off method");
      await child.wait();
    });

    it("should have emit method", async () => {
      const child = child_process.spawn("echo", ["test"]);
      assert.ok(typeof child.emit === "function", "should have emit method");
      await child.wait();
    });
  });
});
