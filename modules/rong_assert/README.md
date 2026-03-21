# rong_assert

Provides assertion utilities for runtime value checking.

## JS APIs

- `assert(value, message?)` — assert that a value is truthy
  - `assert.ok(value, message?)` — alias for `assert()`
  - `assert.equal(left, right, message?)` — assert two values are equal
  - `assert.fail(message?)` — force an assertion failure
  - `assert.doesNotThrow(fn, message?)` — assert that a function does not throw
