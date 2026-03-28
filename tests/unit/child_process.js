const isWindows = process.env.OS === "Windows_NT" || !!process.env.ComSpec;
const tempDir = isWindows
  ? (process.env.TEMP || process.env.TMP || "C:\\Windows\\Temp")
  : "/tmp";
const ARG_PREFIX = "RONG_ARG:";
function echoProcess(text) {
  if (isWindows) {
    return ["cmd", ["/C", `echo ${text}`]];
  }
  return ["echo", [text]];
}

function cwdProcess() {
  if (isWindows) {
    return ["cmd", ["/C", "cd"]];
  }
  return ["pwd", []];
}

function stdinEchoProcess() {
  if (isWindows) {
    return [
      "powershell",
      [
        "-NoProfile",
        "-Command",
        "[Console]::Out.Write([Console]::In.ReadToEnd())",
      ],
    ];
  }
  return ["cat", []];
}

function longRunningProcess(seconds) {
  if (isWindows) {
    return [
      "powershell",
      ["-NoProfile", "-Command", `Start-Sleep -Seconds ${seconds}`],
    ];
  }
  return ["sleep", [String(seconds)]];
}

function echoFileProcess(...parts) {
  if (isWindows) {
    return [process.env.ComSpec || "cmd.exe", ["/C", "echo", ...parts]];
  }
  return ["/bin/echo", parts];
}

function exactArgsFileProcess(...parts) {
  if (isWindows) {
    return [
      "powershell",
      [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        "tests/unit/print-args.ps1",
        ...parts,
      ],
    ];
  }
  return [
    "/bin/sh",
    [
      "tests/unit/print-args.sh",
      ...parts,
    ],
  ];
}

function cwdFileProcess() {
  if (isWindows) {
    return ["cmd", ["/C", "cd"]];
  }
  return ["/bin/pwd", []];
}

function timeoutFileProcess(seconds) {
  if (isWindows) {
    return [
      "powershell",
      ["-NoProfile", "-Command", `Start-Sleep -Seconds ${seconds}`],
    ];
  }
  return ["/bin/sleep", [String(seconds)]];
}

function shellEnvEcho(name) {
  return isWindows ? `echo %${name}%` : `echo $${name}`;
}

function shellPwd() {
  return isWindows ? "cd" : "pwd";
}

function shellSleep(seconds) {
  if (isWindows) {
    return `ping -n ${seconds + 1} 127.0.0.1 > nul`;
  }
  return `sleep ${seconds}`;
}

function splitOutputLines(output) {
  return output
    .replace(/\r/g, "")
    .replace(/\u0000/g, "")
    .split("\n")
    .filter((line, index, lines) => !(index === lines.length - 1 && line === ""));
}

function extractPrintedArgs(output) {
  return splitOutputLines(output)
    .map((line) => {
      const index = line.indexOf(ARG_PREFIX);
      return index >= 0 ? line.slice(index + ARG_PREFIX.length) : null;
    })
    .filter((line) => line !== null);
}

async function countWindowsProcessesWithMarker(marker) {
  if (!isWindows) {
    return 0;
  }

  const result = await child_process.execFile("powershell", [
    "-NoProfile",
    "-Command",
    "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $marker = $args[0]; $self = $PID; (Get-CimInstance Win32_Process | Where-Object { $_.ProcessId -ne $self -and $_.CommandLine -like ('*' + $marker + '*') }).Count",
    marker,
  ]);

  return Number.parseInt(result.stdout.trim() || "0", 10) || 0;
}

describe("Child Process", () => {
  it("hides internal child process classes from global scope", async () => {
    assert.equal(typeof ChildProcess, "undefined");
    assert.equal(typeof ExecResult, "undefined");

    const [command, args] = echoProcess("hello");
    const child = child_process.spawn(command, args);
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
      const result = await child_process.exec(shellPwd(), {
        cwd: tempDir,
      });
      assert.ok(
        result.stdout.toLowerCase().includes(tempDir.toLowerCase()),
        `should execute in ${tempDir}`,
      );
    });

    it("should support env option", async () => {
      const result = await child_process.exec(shellEnvEcho("TEST_CHILD_VAR"), {
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
      const result = await child_process.exec(shellEnvEcho("CHILD_PROCESS_TEST_VAR"));
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
      const marker = `rong_child_timeout_${Date.now()}`;
      try {
        const command = isWindows
          ? `powershell -NoProfile -Command "Start-Sleep -Seconds 10 # ${marker}"`
          : shellSleep(10);
        await child_process.exec(command, { timeout: 100 });
        assert.fail("should have timed out");
      } catch (e) {
        const elapsed = Date.now() - start;
        assert.ok(
          elapsed < (isWindows ? 3000 : 1000),
          "should timeout quickly",
        );
        assert.ok(
          e.message.includes("timed out") || e.code === "ETIMEDOUT",
          "should have timeout error",
        );
        if (isWindows) {
          await new Promise((resolve) => setTimeout(resolve, 200));
          const remaining = await countWindowsProcessesWithMarker(marker);
          assert.equal(remaining, 0, "timeout should terminate the full process tree");
        }
      }
    });

    it("should complete before timeout", async () => {
      const result = await child_process.exec("echo fast", { timeout: 5000 });
      assert.ok(result.stdout.includes("fast"), "should complete successfully");
    });
  });

  describe("execFile", () => {
    it("should execute a file directly", async () => {
      const [file, args] = echoFileProcess("hello", "world");
      const result = await child_process.execFile(file, args);
      assert.ok(
        result.stdout.includes("hello") && result.stdout.includes("world"),
        "stdout should contain both arguments",
      );
      assert.equal(result.code, 0, "exit code should be 0");
    });

    it("should pass arguments correctly", async () => {
      const [file, args] = exactArgsFileProcess("test");
      const result = await child_process.execFile(file, args);
      expect(extractPrintedArgs(result.stdout)).toEqual(["test"]);
    });

    it("should support cwd option", async () => {
      const [file, args] = cwdFileProcess();
      const result = await child_process.execFile(file, args, { cwd: tempDir });
      assert.ok(
        result.stdout.toLowerCase().includes(tempDir.toLowerCase()),
        `should execute in ${tempDir}`,
      );
    });

    it("should handle special characters in arguments", async () => {
      const expected = ["hello world", "foo'bar", 'baz"qux'];
      const [file, args] = exactArgsFileProcess(...expected);
      const result = await child_process.execFile(file, args);
      expect(extractPrintedArgs(result.stdout)).toEqual(expected);
    });

    it("should support timeout option", async () => {
      const start = Date.now();
      try {
        const [file, args] = timeoutFileProcess(10);
        await child_process.execFile(file, args, { timeout: 100 });
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
      const [command, args] = echoProcess("hello");
      const child = child_process.spawn(command, args);
      assert.ok(child !== null, "should return a ChildProcess object");
      assert.ok(typeof child.pid === "number", "should have pid property");
      await child.wait();
    });

    it("should have stdin, stdout and stderr streams", async () => {
      const [command, args] = stdinEchoProcess();
      const child = child_process.spawn(command, args);
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
      const [command, args] = stdinEchoProcess();
      const child = child_process.spawn(command, args);

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
      const [command, args] = echoProcess("hello streaming");
      const child = child_process.spawn(command, args);
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
      const child = child_process.spawn(
        isWindows ? "echo %USERPROFILE%" : "echo $HOME",
        [],
        { shell: true },
      );
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
      const [command, args] = cwdProcess();
      const child = child_process.spawn(command, args, { cwd: tempDir });
      const reader = child.stdout.getReader();
      let output = "";
      const decoder = new TextDecoder();

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        output += decoder.decode(value);
      }

      assert.ok(
        output.toLowerCase().includes(tempDir.toLowerCase()),
        `should execute in ${tempDir}`,
      );
    });

    it("should get exitCode via wait()", async () => {
      const [command, args] = echoProcess("test");
      const child = child_process.spawn(command, args);
      const code = await child.wait();
      assert.equal(code, 0, "exitCode should be 0");
    });

    it("should capture non-zero exit code via wait()", async () => {
      const child = child_process.spawn("exit", ["42"], { shell: true });
      const code = await child.wait();
      assert.equal(code, 42, "exitCode should be 42");
    });

    it("should emit exit event", async () => {
      const [command, args] = echoProcess("test");
      const child = child_process.spawn(command, args);
      const code = await new Promise((resolve) => {
        child.once("exit", resolve);
      });
      assert.equal(code, 0, "exit event should provide exit code");
    });
  });

  describe("ChildProcess properties", () => {
    it("should have pid after spawn", async () => {
      const [command, args] = echoProcess("test");
      const child = child_process.spawn(command, args);
      assert.ok(typeof child.pid === "number", "pid should be a number");
      assert.ok(child.pid > 0, "pid should be positive");
      await child.wait();
    });

    it("should have kill method", async () => {
      const [command, args] = longRunningProcess(10);
      const child = child_process.spawn(command, args);
      assert.ok(typeof child.kill === "function", "should have kill method");
      // Kill the sleep process with default SIGTERM
      const result = child.kill();
      assert.equal(result, true, "kill should return true for running process");
      await child.wait();
    });

    it("should support kill with signal name", async () => {
      const [command, args] = longRunningProcess(10);
      const child = child_process.spawn(command, args);
      const result = child.kill("SIGKILL");
      assert.equal(result, true, "kill with SIGKILL should return true");
      await child.wait();
    });

    it("should support kill with signal number", async () => {
      const [command, args] = longRunningProcess(10);
      const child = child_process.spawn(command, args);
      const result = child.kill("9"); // SIGKILL
      assert.equal(result, true, "kill with signal number should return true");
      await child.wait();
    });

    it("should have wait method", async () => {
      const [command, args] = echoProcess("test");
      const child = child_process.spawn(command, args);
      assert.ok(typeof child.wait === "function", "should have wait method");
      await child.wait();
    });
  });

  describe("EventEmitter interface", () => {
    it("should have on method", async () => {
      const [command, args] = echoProcess("test");
      const child = child_process.spawn(command, args);
      assert.ok(typeof child.on === "function", "should have on method");
      await child.wait();
    });

    it("should have once method", async () => {
      const [command, args] = echoProcess("test");
      const child = child_process.spawn(command, args);
      assert.ok(typeof child.once === "function", "should have once method");
      await child.wait();
    });

    it("should have off method", async () => {
      const [command, args] = echoProcess("test");
      const child = child_process.spawn(command, args);
      assert.ok(typeof child.off === "function", "should have off method");
      await child.wait();
    });

    it("should have emit method", async () => {
      const [command, args] = echoProcess("test");
      const child = child_process.spawn(command, args);
      assert.ok(typeof child.emit === "function", "should have emit method");
      await child.wait();
    });
  });
});
