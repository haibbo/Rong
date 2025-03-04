class TestRunner {
  constructor() {
    this.tests = [];
    this.passed = 0;
    this.failed = 0;
    this.afterEachCallbacks = [];
    this.beforeEachCallbacks = [];
    this.currentSuite = null;
    this.testCount = 0;
  }

  describe(name, fn) {
    this.currentSuite = { name, tests: [] };
    this.tests.push(this.currentSuite);
    fn();
    this.currentSuite = null;
  }

  it(name, fn) {
    if (!this.currentSuite) {
      throw new Error("it() must be called within a describe() block");
    }
    this.testCount++;
    this.currentSuite.tests.push({ name, fn, number: this.testCount });
  }

  async runTests() {
    for (const suite of this.tests) {
      console.log(`\nRunning suite: ${suite.name}`);
      for (const test of suite.tests) {
        console.log(`${test.number}. ${test.name}...`);
        try {
          if (this.beforeEachCallbacks) {
            await Promise.all(this.beforeEachCallbacks.map((cb) => cb()));
          }

          // Handle async tests
          await new Promise(async (resolve, reject) => {
            let isDone = false;
            const done = () => {
              isDone = true;
              resolve();
            };

            try {
              // Call test function with done callback
              const result = test.fn(done);
              // If it returns a Promise, wait for it
              if (result && typeof result.then === "function") {
                await result;
                if (!isDone) done();
              } else if (!test.fn.length) {
                // If function has no parameters (doesn't need done callback)
                done();
              }

              // let rust test to handler timeout
              // setTimeout(() => {
              //   if (!isDone) {
              //     reject(new Error("Test timeout after 1000ms"));
              //   }
              // }, 1000);
            } catch (e) {
              reject(e);
            }
          });

          this.passed++;
          console.log("    ✓ Passed");
        } catch (e) {
          this.failed++;
          console.log("    ✗ Failed");
          if (e.message) {
            console.log(`      Error: ${e.message}`);
          }
          if (e.stack) {
            console.log(e.stack.split("\n").slice(1).join("\n"));
          }
        } finally {
          await Promise.all(this.afterEachCallbacks.map((cb) => cb()));
        }
      }
    }

    console.log(`\nTest Results:`);
    console.log(`  Passed: ${this.passed}`);
    console.log(`  Failed: ${this.failed}`);

    return this.failed === 0;
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
            throw new Error(
              `Expected string to contain "${expected}", got "${value}"`,
            );
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
        if (value instanceof Uint8Array && expected instanceof Uint8Array) {
          if (value.length !== expected.length) {
            throw new Error(
              `Expected Uint8Array length ${expected.length}, but got ${value.length}`,
            );
          }
          for (let i = 0; i < value.length; i++) {
            if (value[i] !== expected[i]) {
              throw new Error(
                `Expected Uint8Array element at index ${i} to be ${expected[i]}, but got ${value[i]}`,
              );
            }
          }
        } else if (Array.isArray(value) && Array.isArray(expected)) {
          if (value.length !== expected.length) {
            throw new Error(
              `Expected array length ${expected.length}, but got ${value.length}`,
            );
          }
          for (let i = 0; i < value.length; i++) {
            if (value[i] !== expected[i]) {
              throw new Error(
                `Expected array element at index ${i} to be ${expected[i]}, but got ${value[i]}`,
              );
            }
          }
        } else if (value !== expected) {
          throw new Error(`Expected ${expected}, but got ${value}`);
        }
      },
      toThrow: (expectedReasonOrType) => {
        try {
          value();
          return {
            pass: false,
            message: () => `Expected function to throw, but it did not.`,
          };
        } catch (error) {
          // If the input is an Error type (e.g. TypeError)
          if (
            typeof expectedReasonOrType === "function" &&
            expectedReasonOrType.prototype instanceof Error
          ) {
            const pass = error instanceof expectedReasonOrType;
            return {
              pass,
              message: () =>
                pass
                  ? `Expected function not to throw ${expectedReasonOrType.name}, but it threw: ${error}`
                  : `Expected function to throw ${expectedReasonOrType.name}, but it threw: ${error}`,
            };
          }

          // If no reason or type is specified, just check if an error was thrown
          if (expectedReasonOrType === undefined) {
            return {
              pass: true,
              message: () =>
                `Expected function not to throw, but it threw: ${error}`,
            };
          }

          // Use assert.equal for shallow comparison
          let pass = false;
          try {
            assert.equal(error, expectedReasonOrType);
            pass = true;
          } catch (e) {
            pass = false;
          }

          return {
            pass,
            message: () =>
              pass
                ? `Expected function not to throw ${expectedReasonOrType}, but it threw: ${error}`
                : `Expected function to throw ${expectedReasonOrType}, but it threw: ${error}`,
          };
        }
      },
      toBeUndefined: () => {
        if (value !== undefined) {
          throw new Error(`Expected value to be undefined, got ${value}`);
        }
      },
      toBeInstanceOf(expected) {
        if (!(value instanceof expected)) {
          throw new Error(
            `Expected value to be an instance of ${expected.name}, but got ${value.constructor.name}`,
          );
        }
      },
    };
  }
}

const runner = new TestRunner();

globalThis.describe = (name, fn) => runner.describe(name, fn);
globalThis.it = (name, fn) => runner.it(name, fn);
globalThis.expect = (value) => runner.expect(value);
globalThis.afterEach = (fn) => runner.afterEach(fn);
globalThis.beforeEach = (fn) => runner.beforeEach(fn);

// Export an async function that waits for all tests to complete
globalThis.runTests = () => runner.runTests();
