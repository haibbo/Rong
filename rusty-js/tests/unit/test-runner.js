class TestRunner {
  constructor() {
    this.passed = 0;
    this.failed = 0;
  }

  describe(name, fn) {
    console.log(`\nRunning suite: ${name}`);
    fn();
  }

  it(name, fn) {
    let testNumber = this.passed + this.failed + 1;
    console.log(`${testNumber}. ${name}... `);
    try {
      fn();
      this.passed++;
      console.log("    ✓ Passed");
    } catch (e) {
      this.failed++;
      console.log("    ✗ Failed");
      console.error(`      Error: ${e.message}`);
      if (e.stack) {
        console.error(e.stack.split("\n").slice(1).join("\n"));
      }
    }
  }

  expect(value) {
    return {
      toThrow: (expectedError) => {
        let threw = false;
        try {
          value();
        } catch (e) {
          threw = true;
          if (expectedError && !(e instanceof expectedError)) {
            throw new Error(
              `Expected ${expectedError.name} but got ${e.constructor.name}`,
            );
          }
        }
        if (!threw) {
          throw new Error("Expected function to throw but it did not");
        }
      },
      toContain: (expected) => {
        if (!value.includes(expected)) {
          throw new Error(`Expected "${value}" to contain "${expected}"`);
        }
      },
    };
  }

  report() {
    console.log("\nTest Results:");
    console.log(`  Passed: ${this.passed}`);
    console.log(`  Failed: ${this.failed}`);

    return {
      passed: this.passed,
      failed: this.failed,
    };
  }
}

const runner = new TestRunner();

globalThis.describe = (name, fn) => runner.describe(name, fn);
globalThis.it = (name, fn) => runner.it(name, fn);
globalThis.expect = (value) => runner.expect(value);
