// Simple assertion functions
function assert(condition, message) {
  if (!condition) {
    console.log("Assertion failed: " + message);
    throw message || "Assertion failed";
  }
  return true;
}

function assertEqual(actual, expected, message) {
  if (actual != expected) {
    let error = `Expected value "${expected}" but got "${actual}"${message ? ": " + message : ""}`;
    console.log("Assertion failed: " + error);
    throw error;
  }
  return true;
}

// Test results storage
const results = {
  total: 0,
  passed: 0,
  failed: [],
};

// Run tests and return results
(async function () {
  try {
    // Test constructor
    results.total += 3;
    console.log("Testing constructor...");
    const h1 = new Headers();
    results.passed += assert(
      h1 instanceof Headers,
      "Empty Headers should be instance of Headers",
    );

    // Test constructor with object
    const h2 = new Headers({
      "Content-Type": "text/plain",
      "X-Custom": "test",
    });
    results.passed += assert(
      h2.get("content-type") === "text/plain",
      "Headers should accept object initialization",
    );

    // Test constructor with array
    const h3 = new Headers([
      ["content-type", "text/plain"],
      ["x-custom", "test"],
    ]);
    results.passed += assert(
      h3.get("x-custom") === "test",
      "Headers should accept array initialization",
    );

    // Test basic operations
    results.total += 4;
    console.log("Testing basic operations...");

    // Test set/get
    const headers = new Headers();
    headers.set("Content-Type", "text/plain");
    results.passed += assert(
      headers.get("content-type") === "text/plain",
      "get should return set value",
    );

    // Test case insensitivity
    results.passed += assert(
      headers.get("CONTENT-TYPE") === "text/plain",
      "Headers should be case-insensitive",
    );

    // Test has
    results.passed += assert(
      headers.has("content-type"),
      "has should return true for existing header",
    );

    // Test delete
    headers.delete("content-type");
    results.passed += assert(
      !headers.has("content-type"),
      "delete should remove header",
    );

    // Test append
    results.total += 2;
    console.log("Testing append...");
    headers.append("accept", "text/html");
    headers.append("accept", "application/json");
    results.passed += assert(
      headers.get("accept").includes("text/html"),
      "append should add first value",
    );
    results.passed += assert(
      headers.get("accept").includes("application/json"),
      "append should add second value",
    );

    // Test invalid inputs
    results.total += 4;
    console.log("Testing invalid inputs...");

    // Test invalid header name
    let hasError = false;
    try {
      headers.set("", "value");
    } catch (e) {
      hasError = true;
      if (!(e instanceof TypeError)) {
        console.log(`Caught non-TypeError: ${e.constructor.name}`);
      }
      results.passed += hasError;
    }
    if (!hasError) {
      throw "Should throw TypeError for empty header name";
    }

    // Test invalid header value
    hasError = false;
    try {
      const valueWithNull = "test" + String.fromCharCode(0) + "value";
      headers.set("test", valueWithNull);
    } catch (e) {
      hasError = true;
      if (!(e instanceof TypeError)) {
        console.log(`Caught non-TypeError: ${e.constructor.name}`);
      }
      results.passed += hasError;
    }
    if (!hasError) {
      throw "Should throw TypeError for invalid header value.";
    }

    // Test invalid constructor input
    hasError = false;
    try {
      new Headers("invalid");
    } catch (e) {
      hasError = true;
      if (!(e instanceof TypeError)) {
        console.log(`Caught non-TypeError: ${e.constructor.name}`);
      }
      results.passed += hasError;
    }
    if (!hasError) {
      throw "Should throw TypeError for invalid constructor input";
    }

    // Test invalid array format
    hasError = false;
    try {
      new Headers([["invalid"]]);
    } catch (e) {
      hasError = true;
      if (!(e instanceof TypeError)) {
        console.log(`Caught non-TypeError: ${e.constructor.name}`);
      }
      results.passed += hasError;
    }
    if (!hasError) {
      throw "Should throw TypeError for invalid array format";
    }

    // Test iteration methods
    results.total += 3;
    console.log("Testing iteration methods...");

    const iterHeaders = new Headers({
      "Content-Type": "text/plain",
      "X-Custom": "test",
      Accept: "application/json",
    });

    // Test entries()
    const entries = Array.from(iterHeaders.entries());
    results.passed += assert(
      entries.length === 3,
      "entries() should return all headers",
    );

    // Test keys()
    const keys = Array.from(iterHeaders.keys());
    results.passed += assert(
      keys.length === 3,
      "keys() should return all header names",
    );

    // Test values()
    const values = Array.from(iterHeaders.values());
    results.passed += assert(
      values.length === 3,
      "values() should return all header values",
    );

    // Test forEach
    results.total += 2;
    console.log("Testing forEach...");
    let count = 0;
    let containerValid = true;

    iterHeaders.forEach((value, key, container) => {
      count++;
      containerValid = containerValid && container instanceof Headers;
    });

    results.passed += assert(
      containerValid,
      "forEach should pass Headers instance as third argument",
    );

    results.passed += assert(count === 3, "forEach should iterate all headers");

    // Test Set-Cookie handling
    results.total += 2;
    console.log("Testing Set-Cookie handling...");
    const cookieHeaders = new Headers();
    cookieHeaders.append("Set-Cookie", "cookie1=value1");
    cookieHeaders.append("Set-Cookie", "cookie2=value2");

    const cookies = cookieHeaders.getSetCookie();
    results.passed += assert(
      Array.isArray(cookies),
      "getSetCookie should return array",
    );
    results.passed += assert(
      cookies.length === 2,
      "getSetCookie should return all cookies",
    );

    console.log(`Tests completed: ${results.passed}/${results.total} passed`);
  } catch (err) {
    console.log("Test failed with error: " + err);
    console.log(
      "Error details:",
      err instanceof Error ? err.stack : "No stack trace available",
    );
    results.failed.push(err.toString());
  }

  return {
    total: results.total,
    passed: results.passed,
    failed: results.failed,
    success: results.failed.length === 0 && results.total === results.passed,
  };
})();
