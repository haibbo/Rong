class TestRunner {
  constructor() {
    this.tests = [];
    this.passed = 0;
    this.failed = 0;
    this.afterEachCallbacks = [];
    this.beforeEachCallbacks = [];
  }

  describe(name, fn) {
    console.log(`\nRunning suite: ${name}`);
    fn();
  }

  it(name, fn) {
    let testNumber = this.passed + this.failed + 1;
    console.log(`${testNumber}. ${name}... `);
    try {
      if (this.beforeEachCallbacks) {
        this.beforeEachCallbacks.forEach((callback) => callback());
      }
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
    } finally {
      this.afterEachCallbacks.forEach((callback) => callback());
    }
  }

  afterEach(fn) {
    this.afterEachCallbacks.push(fn);
  }

  beforeEach(fn) {
    this.beforeEachCallbacks = this.beforeEachCallbacks || [];
    this.beforeEachCallbacks.push(fn);
  }

  expect(value) {
    return {
      toBe: (expected) => {
        if (value !== expected) {
          throw new Error(`Expected ${expected}, got ${value}`);
        }
      },
      toContain: (expected) => {
        if (Array.isArray(value)) {
          if (!value.includes(expected)) {
            throw new Error(`Expected array to contain ${expected}`);
          }
        } else if (typeof value === "string") {
          if (!value.includes(expected)) {
            throw new Error(`Expected string to contain "${expected}"`);
          }
        } else {
          throw new Error("toContain can only be used with arrays or strings");
        }
      },
      toBeTruthy: () => {
        if (!value) {
          throw new Error(`Expected value to be truthy, got ${value}`);
        }
      },
      toBeFalsy: () => {
        if (value) {
          throw new Error(`Expected value to be falsy, got ${value}`);
        }
      },
      toEqual: (expected) => {
        if (value !== expected) {
          throw new Error(`Expected ${expected}, got ${value}`);
        }
      },
      toThrow: (expectedError) => {
        try {
          value();
          throw new Error("Expected function to throw an error but it did not");
        } catch (e) {
          if (
            typeof expectedError === "function" &&
            !(e instanceof expectedError)
          ) {
            throw new Error(
              `Expected error to be instance of ${expectedError.name}, but got ${e.constructor.name}`,
            );
          } else if (
            typeof expectedError === "string" &&
            e.message !== expectedError
          ) {
            throw new Error(
              `Expected error message "${expectedError}" but got "${e.message}"`,
            );
          }
        }
      },
      toBeUndefined: () => {
        if (value !== undefined) {
          throw new Error(`Expected value to be undefined, got ${value}`);
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
      success: this.failed === 0,
    };
  }
}

const runner = new TestRunner();

globalThis.describe = (name, fn) => runner.describe(name, fn);
globalThis.it = (name, fn) => runner.it(name, fn);
globalThis.expect = (value) => runner.expect(value);
globalThis.afterEach = (fn) => runner.afterEach(fn);
globalThis.beforeEach = (fn) => runner.beforeEach(fn);
