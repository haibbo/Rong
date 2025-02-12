// Simple assertion functions
function assert(condition, message) {
  if (!condition) {
    print("Assertion failed: " + message);
    throw message || "Assertion failed";
  }
  return true;
}

function assertEqual(actual, expected, message) {
  if (actual != expected) {
    let error = `Expected value "${expected}" but got "${actual}"${message ? ": " + message : ""}`;
    print("Assertion failed: " + error);
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
    // Test constructor with URL string
    results.total += 3;
    print("Testing constructor with URL string...");
    const request = new Request("https://example.com");
    results.passed += assert(
      request.method === "GET",
      "Default method should be GET",
    );
    results.passed += assert(
      request.url === "https://example.com",
      "URL should be set correctly",
    );
    results.passed += assert(
      request.headers instanceof Headers,
      "Headers should be instance of Headers",
    );

    // Test constructor with init options
    results.total += 5;
    print("Testing constructor with init options...");
    const requestWithInit = new Request("https://example.com", {
      method: "POST",
      mode: "no-cors",
      credentials: "include",
      cache: "no-cache",
      redirect: "error",
      headers: {
        "Content-Type": "application/json",
      },
    });

    results.passed += assert(
      requestWithInit.method === "POST",
      "Method should be set from init",
    );
    results.passed += assert(
      requestWithInit.mode === "no-cors",
      "Mode should be set from init",
    );
    results.passed += assert(
      requestWithInit.credentials === "include",
      "Credentials should be set from init",
    );
    results.passed += assert(
      requestWithInit.cache === "no-cache",
      "Cache should be set from init",
    );
    results.passed += assert(
      requestWithInit.headers.get("content-type") === "application/json",
      "Headers should be set from init",
    );

    // Test constructor with Request object
    results.total += 3;
    print("Testing constructor with Request object...");
    const original = new Request("https://example.com", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
    });
    const copy = new Request(original);

    results.passed += assert(
      copy.method === original.method,
      "Method should be copied",
    );
    results.passed += assert(copy.url === original.url, "URL should be copied");
    results.passed += assert(
      copy.headers.get("content-type") === original.headers.get("content-type"),
      "Headers should be copied",
    );

    // Test clone method
    results.total += 3;
    print("Testing clone method...");
    const cloned = original.clone();
    results.passed += assert(
      cloned.method === original.method,
      "Cloned method should match",
    );
    results.passed += assert(
      cloned.url === original.url,
      "Cloned URL should match",
    );
    results.passed += assert(
      cloned.headers.get("content-type") ===
        original.headers.get("content-type"),
      "Cloned headers should match",
    );

    // Test invalid inputs
    results.total += 6;
    print("Testing invalid inputs...");
    let hasError = false;

    try {
      new Request("not-a-url");
    } catch (e) {
      hasError = true;
      results.passed += assert(
        e instanceof TypeError,
        "Invalid URL should throw TypeError",
      );
    }
    if (!hasError) throw "Should throw error for invalid URL";

    hasError = false;
    try {
      new Request("https://example.com", { method: "INVALID" });
    } catch (e) {
      hasError = true;
      results.passed += assert(
        e instanceof TypeError,
        "Invalid method should throw TypeError",
      );
    }
    if (!hasError) throw "Should throw error for invalid method";

    // Test other invalid options
    ["mode", "credentials", "cache", "redirect"].forEach((option) => {
      hasError = false;
      try {
        const init = {};
        init[option] = "invalid-value";
        new Request("https://example.com", init);
      } catch (e) {
        hasError = true;
        results.passed += assert(
          e instanceof TypeError,
          `Invalid ${option} should throw TypeError`,
        );
      }
      if (!hasError) throw `Should throw error for invalid ${option}`;
    });

    // Test getters
    results.total += 13;
    print("Testing getters...");
    const requestWithAllProps = new Request("https://example.com", {
      method: "POST",
      mode: "no-cors",
      credentials: "include",
      cache: "no-cache",
      redirect: "error",
      referrer: "https://example.com/referrer",
      referrerPolicy: "strict-origin",
      integrity: "sha256-hash",
      keepalive: true,
      headers: {
        "Content-Type": "application/json",
      },
    });

    results.passed += assert(
      requestWithAllProps.destination === "",
      "Destination should be empty string",
    );
    results.passed += assert(
      requestWithAllProps.referrer === "https://example.com/referrer",
      "Referrer should match",
    );
    results.passed += assert(
      requestWithAllProps.referrerPolicy === "strict-origin",
      "ReferrerPolicy should match",
    );
    results.passed += assert(
      requestWithAllProps.mode === "no-cors",
      "Mode should match",
    );
    results.passed += assert(
      requestWithAllProps.credentials === "include",
      "Credentials should match",
    );
    results.passed += assert(
      requestWithAllProps.cache === "no-cache",
      "Cache should match",
    );
    results.passed += assert(
      requestWithAllProps.redirect === "error",
      "Redirect should match",
    );
    results.passed += assert(
      requestWithAllProps.integrity === "sha256-hash",
      "Integrity should match",
    );
    results.passed += assert(
      requestWithAllProps.keepalive === true,
      "Keepalive should match",
    );
    results.passed += assert(
      requestWithAllProps.isReloadNavigation === false,
      "isReloadNavigation should be false by default",
    );
    results.passed += assert(
      requestWithAllProps.isHistoryNavigation === false,
      "isHistoryNavigation should be false by default",
    );
    results.passed += assert(
      requestWithAllProps.signal === null,
      "Signal should be null by default",
    );
    results.passed += assert(
      requestWithAllProps.body === null,
      "Body should be null by default",
    );

    // Test async methods
    results.total += 2;
    print("Testing async methods...");

    // Create a request with text body
    const textBlob = new Blob(["Hello World"], { type: "text/plain" });
    const requestWithBody = new Request("https://example.com", {
      method: "POST",
      body: textBlob,
    });

    // Test text() method
    const text = await requestWithBody.text();
    results.passed += assert(
      text === "Hello World",
      "text() should return correct content",
    );

    // Test json() method
    // const jsonBlob = new Blob(['{"message":"Hello"}'], {
    //   type: "application/json",
    // });
    // const jsonRequest = new Request("https://example.com", {
    //   method: "POST",
    //   body: jsonBlob,
    // });
    // const json = await jsonRequest.json();
    // results.passed += assert(
    //   json.message === "Hello",
    //   "json() should parse JSON correctly",
    // );

    // Test arrayBuffer() method
    const buffer = await requestWithBody.arrayBuffer();
    const view = new Uint8Array(buffer);
    const decoder = new TextDecoder();
    results.passed += assert(
      decoder.decode(view) === "Hello World",
      "arrayBuffer() should return correct content",
    );

    // Test formData() method - should throw not implemented error
    // let hasFormDataError = false;
    // try {
    //   await requestWithBody.formData();
    // } catch (e) {
    //   hasFormDataError =
    //     e instanceof TypeError && e.message.includes("Not implemented");
    //   results.passed += hasFormDataError;
    // }
    // if (!hasFormDataError)
    //   throw "formData() should throw not implemented error";

    print(`Tests completed: ${results.passed}/${results.total} passed`);
  } catch (err) {
    print("Test failed with error: " + err);
    print(
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
