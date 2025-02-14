// Test framework setup
let total = 0;
let passed = 0;
let failed = [];

function assert(condition, message) {
  total++;
  if (condition) {
    passed++;
    console.log(`✓ ${message}`);
  } else {
    failed.push(message);
    console.log(`✗ ${message}`);
  }
}

// Test Event constructor and properties
{
  const event = new Event("test");
  assert(event.type === "test", "Event type should match");
  assert(!event.bubbles, "Event should not bubble by default");
  assert(!event.cancelable, "Event should not be cancelable by default");
  assert(!event.composed, "Event should not be composed by default");

  const customEvent = new Event("custom", {
    bubbles: true,
    cancelable: true,
    composed: true,
  });
  assert(customEvent.bubbles, "Event should bubble when specified");
  assert(customEvent.cancelable, "Event should be cancelable when specified");
  assert(customEvent.composed, "Event should be composed when specified");
}

// Test EventTarget basic functionality
{
  const target = new EventTarget();
  let called = false;

  const listener = () => {
    called = true;
  };

  target.addEventListener("test", listener);
  target.dispatchEvent(new Event("test"));
  assert(called, "Basic event listener should be called");
}

// Test once option
{
  const target = new EventTarget();
  let count = 0;

  const onceListener = () => {
    count++;
  };

  // Test with explicit once option
  target.addEventListener("once-test", onceListener, { once: true });
  target.dispatchEvent(new Event("once-test"));
  assert(count === 1, "Once listener should be called first time");

  target.dispatchEvent(new Event("once-test"));
  assert(count === 1, "Once listener should not be called second time");

  // Test once with multiple listeners
  const target2 = new EventTarget();
  let sequence = [];

  target2.addEventListener("multi-once", () => sequence.push(1), {
    once: true,
  });
  target2.addEventListener("multi-once", () => sequence.push(2));
  target2.addEventListener("multi-once", () => sequence.push(3), {
    once: true,
  });

  target2.dispatchEvent(new Event("multi-once"));
  assert(
    sequence.join(",") === "1,2,3",
    "Multiple listeners should be called in order",
  );

  target2.dispatchEvent(new Event("multi-once"));
  assert(
    sequence.join(",") === "1,2,3,2",
    "Only non-once listener should be called second time",
  );
}

// Test removeEventListener
{
  const target = new EventTarget();
  let count = 0;

  const listener = () => {
    count++;
  };

  target.addEventListener("remove-test", listener);
  target.dispatchEvent(new Event("remove-test"));
  target.removeEventListener("remove-test", listener);
  target.dispatchEvent(new Event("remove-test"));

  assert(count === 1, "Listener should be removed");

  // Test removeEventListener with capture option
  let captureCount = 0;
  const captureListener = () => {
    captureCount++;
  };

  target.addEventListener("capture-remove-test", captureListener, {
    capture: true,
  });
  target.dispatchEvent(new Event("capture-remove-test"));
  target.removeEventListener("capture-remove-test", captureListener, {
    capture: false,
  });
  target.dispatchEvent(new Event("capture-remove-test"));

  assert(
    captureCount === 2,
    "Listener should not be removed with different capture value",
  );

  target.removeEventListener("capture-remove-test", captureListener, {
    capture: true,
  });
  target.dispatchEvent(new Event("capture-remove-test"));

  assert(
    captureCount === 2,
    "Listener should be removed with matching capture value",
  );
}

// Test multiple listeners
{
  const target = new EventTarget();
  let results = [];

  target.addEventListener("multi-test", () => results.push(1));
  target.addEventListener("multi-test", () => results.push(2));
  target.addEventListener("multi-test", () => results.push(3));

  target.dispatchEvent(new Event("multi-test"));
  assert(
    results.join(",") === "1,2,3",
    "Multiple listeners should be called in order",
  );
}

// Test duplicate event listeners
{
  const target = new EventTarget();
  let count = 0;
  const listener = () => count++;

  // Add same listener multiple times - should only be added once
  target.addEventListener("duplicate-test", listener);
  target.addEventListener("duplicate-test", listener);
  target.addEventListener("duplicate-test", listener);

  target.dispatchEvent(new Event("duplicate-test"));
  assert(count === 1, "Same listener should only be registered once");

  // Remove the listener
  target.removeEventListener("duplicate-test", listener);
  count = 0;
  target.dispatchEvent(new Event("duplicate-test"));
  assert(
    count === 0,
    "Listener should be completely removed after removeEventListener",
  );

  // Test with different options
  const target2 = new EventTarget();
  let count2 = 0;
  const listener2 = () => count2++;

  target2.addEventListener("duplicate-test", listener2, { capture: true });
  target2.addEventListener("duplicate-test", listener2, { capture: false });

  target2.dispatchEvent(new Event("duplicate-test"));
  assert(
    count2 === 2,
    "Same listener with different options should be treated as different listeners",
  );
}

// Test event object properties in listener
{
  const target = new EventTarget();
  let eventInListener = null;

  const listener = (e) => {
    eventInListener = e;
  };

  // Test basic event properties
  target.addEventListener("prop-test", listener);
  const originalEvent = new Event("prop-test", {
    bubbles: true,
    cancelable: true,
    composed: true,
  });

  target.dispatchEvent(originalEvent);
  assert(eventInListener !== null, "Event object should be passed to listener");
  assert(
    eventInListener.type === "prop-test",
    "Event type should be preserved in listener",
  );
  assert(
    eventInListener.bubbles === true,
    "Event bubbles property should be preserved",
  );
  assert(
    eventInListener.cancelable === true,
    "Event cancelable property should be preserved",
  );
  assert(
    eventInListener.composed === true,
    "Event composed property should be preserved",
  );

  // Test event object immutability
  const target2 = new EventTarget();
  let eventProps = null;

  target2.addEventListener("immutable-test", (e) => {
    // Store initial property values
    eventProps = {
      type: e.type,
      bubbles: e.bubbles,
      cancelable: e.cancelable,
      composed: e.composed,
    };

    // Try to modify properties
    e.type = "modified";
    e.bubbles = !e.bubbles;
    e.cancelable = !e.cancelable;
    e.composed = !e.composed;
  });

  const testEvent = new Event("immutable-test");
  target2.dispatchEvent(testEvent);

  // Verify properties remained unchanged
  assert(
    eventProps.type === "immutable-test" &&
      eventProps.bubbles === false &&
      eventProps.cancelable === false &&
      eventProps.composed === false,
    "Event properties should not be modifiable in listener",
  );
}

// Test event type case sensitivity
{
  const target = new EventTarget();
  let lowerCalled = false;
  let upperCalled = false;

  target.addEventListener("test", () => {
    lowerCalled = true;
  });
  target.addEventListener("TEST", () => {
    upperCalled = true;
  });

  target.dispatchEvent(new Event("test"));
  assert(lowerCalled && !upperCalled, "Event types should be case-sensitive");
}

// Print test summary
console.log(`\nTest Summary:`);
console.log(`Total: ${total}`);
console.log(`Passed: ${passed}`);
console.log(`Failed: ${failed.length}`);
if (failed.length > 0) {
  console.log("\nFailed Tests:");
  failed.forEach((msg, i) => console.log(`${i + 1}. ${msg}`));
}

// Export test results
({
  total,
  passed,
  failed: failed.length,
  success: failed.length === 0,
});
