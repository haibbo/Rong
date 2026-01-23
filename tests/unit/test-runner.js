class TestRunner {
  constructor() {
    this.tests = [];
    this.passed = 0;
    this.failed = 0;
    this.failures = [];
    this.currentSuite = null;
    this.testCount = 0;
    this.suites = new Map(); // Store suites and their hooks
  }

  describe(name, fn) {
    const parentSuite = this.currentSuite;
    const suite = {
      name,
      tests: [],
      beforeEachCallbacks: [],
      afterEachCallbacks: [],
      parent: parentSuite,
    };

    this.suites.set(suite, {
      beforeEach: [],
      afterEach: [],
    });

    this.currentSuite = suite;
    this.tests.push(suite);

    try {
      fn();
    } finally {
      this.currentSuite = parentSuite;
    }
  }

  beforeEach(fn) {
    if (!this.currentSuite) {
      throw new Error("beforeEach must be called within a describe block");
    }
    this.suites.get(this.currentSuite).beforeEach.push(fn);
  }

  afterEach(fn) {
    if (!this.currentSuite) {
      throw new Error("afterEach must be called within a describe block");
    }
    this.suites.get(this.currentSuite).afterEach.push(fn);
  }

  async runTest(suite, test) {
    // Collect all beforeEach hooks from outer to inner
    const beforeEachCallbacks = [];
    let currentSuite = suite;
    while (currentSuite) {
      const hooks = this.suites.get(currentSuite);
      beforeEachCallbacks.unshift(...hooks.beforeEach);
      currentSuite = currentSuite.parent;
    }

    // Collect all afterEach hooks from inner to outer
    const afterEachCallbacks = [];
    currentSuite = suite;
    while (currentSuite) {
      const hooks = this.suites.get(currentSuite);
      afterEachCallbacks.push(...hooks.afterEach);
      currentSuite = currentSuite.parent;
    }

    try {
      // Run all beforeEach hooks
      for (const callback of beforeEachCallbacks) {
        await callback();
      }

      // Run the test with optional 30 second timeout (only if setTimeout is available)
      const testPromise = new Promise(async (resolve, reject) => {
        try {
          // Check if test function expects a done callback (based on function.length)
          if (test.fn.length > 0) {
            // Test uses done callback
            let doneCallCount = 0;
            let settled = false;
            let settleTimer = null;

            const done = (error) => {
              doneCallCount += 1;

              if (doneCallCount > 1) {
                // If we're still pending, fail; if already settled, at least surface it.
                if (!settled) {
                  settled = true;
                  if (settleTimer && typeof clearTimeout === "function") clearTimeout(settleTimer);
                  reject(new Error("done() called multiple times"));
                } else {
                  console.error("done() called multiple times (after test already settled)");
                }
                return;
              }

              if (error) {
                if (!settled) {
                  settled = true;
                  if (settleTimer && typeof clearTimeout === "function") clearTimeout(settleTimer);
                  reject(error);
                }
                return;
              }

              // Delay settle to end of tick so immediate double-calls can be detected.
              // Only use setTimeout if available (timer module might not be loaded)
              if (typeof setTimeout === "function") {
                settleTimer = setTimeout(() => {
                  if (settled) return;
                  settled = true;
                  resolve();
                }, 0);
              } else {
                // No timer available, settle immediately
                if (!settled) {
                  settled = true;
                  resolve();
                }
              }
            };

            // Call test with done callback
            const result = test.fn(done);

            // If test returns a promise, it's using both promise and callback (error)
            if (result && typeof result.then === "function") {
              if (!settled) {
                settled = true;
                if (settleTimer && typeof clearTimeout === "function") clearTimeout(settleTimer);
                reject(new Error("Test uses both promise and done callback"));
              }
            }
          } else {
            // Test uses promise or sync return
            const result = test.fn();
            if (result && typeof result.then === "function") {
              await result;
            }
            resolve();
          }
        } catch (e) {
          reject(e);
        }
      });

      // Add timeout only if setTimeout is available
      if (typeof setTimeout === "function") {
        await Promise.race([
          testPromise,
          new Promise((_, reject) =>
            setTimeout(
              () => reject(new Error(`Test timeout after 30 seconds: ${test.name}`)),
              30000,
            ),
          ),
        ]);
      } else {
        await testPromise;
      }

      this.passed++;
      console.log(`    ✓ Passed`);
    } catch (e) {
      this.failed++;
      this.failures.push({
        suite: suite?.name ?? "<unknown suite>",
        test: test?.name ?? "<unknown test>",
        message: e?.message ?? String(e),
        stack: e?.stack ?? null,
      });
      console.log(`    ✗ Failed`);
      if (e.message) console.log(`      Error: ${e.message}`);
      if (e.stack) console.log(e.stack.split("\n").slice(1).join("\n"));
    } finally {
      // Run all afterEach hooks
      for (const callback of afterEachCallbacks) {
        try {
          await callback();
        } catch (e) {
          console.error("Error in afterEach:", e);
        }
      }
    }
  }

  async runTests() {
    const limit =
      typeof globalThis.__RONG_TEST_LIMIT__ === "number"
        ? globalThis.__RONG_TEST_LIMIT__
        : null;
    const filter =
      typeof globalThis.__RONG_TEST_FILTER__ === "string" &&
      globalThis.__RONG_TEST_FILTER__.length > 0
        ? new RegExp(globalThis.__RONG_TEST_FILTER__)
        : null;

    for (const suite of this.tests) {
      console.log(`\nRunning suite: ${suite.name}`);
      for (const test of suite.tests) {
        if (limit != null && test.number > limit) {
          console.log(`\nTest Results:`);
          console.log(`  Passed: ${this.passed}`);
          console.log(`  Failed: ${this.failed}`);
          return this.failed === 0;
        }

        if (filter && !filter.test(`${suite.name} ${test.name}`)) {
          continue;
        }

        console.log(`${test.number}. ${test.name}...`);
        await this.runTest(suite, test);
      }
    }

    console.log(`\nTest Results:`);
    console.log(`  Passed: ${this.passed}`);
    console.log(`  Failed: ${this.failed}`);

    return this.failed === 0;
  }

  it(name, fn) {
    if (!this.currentSuite) {
      throw new Error("it() must be called within a describe() block");
    }
    this.testCount++;
    this.currentSuite.tests.push({ name, fn, number: this.testCount });
  }

  expect(value) {
    const matchers = {
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

    const negatedMatchers = {
      toBe: (expected) => {
        if (value === expected) {
          throw new Error(`Expected ${value} not to be ${expected}`);
        }
      },
      toContain: (expected) => {
        if (Array.isArray(value)) {
          if (value.includes(expected)) {
            throw new Error(`Expected array not to contain ${expected}`);
          }
        } else if (typeof value === "string") {
          if (value.includes(expected)) {
            throw new Error(
              `Expected string not to contain "${expected}", got "${value}"`,
            );
          }
        } else {
          throw new Error("toContain can only be used with arrays or strings");
        }
      },
      toThrow: (expected) => {
        try {
          value();
          // If we get here, the function didn't throw
        } catch (e) {
          throw new Error(
            `Expected function not to throw, but it threw ${e.message}`,
          );
        }
      },
    };

    return {
      ...matchers,
      not: {
        ...negatedMatchers,
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
