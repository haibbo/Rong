# Assert

Test assertion utilities.

```javascript
assert(true);                    // passes
assert(false, "should be true"); // throws AssertionError

assert.ok(value);                // same as assert(value)
assert.equal(1, 1);              // passes (deep equality)
assert.equal({a: 1}, {a: 1});    // passes
assert.fail("always fails");     // always throws
assert.doesNotThrow(() => 42);   // passes
```
