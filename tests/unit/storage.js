describe("Storage API", () => {
  beforeEach(() => {
    // Clear storage before each test
    Rong.storage.clear();
  });

  it("should set and get string values", () => {
    Rong.storage.set("test_string", "hello world");
    const value = Rong.storage.get("test_string");
    assert.equal(value, "hello world");
  });

  it("should handle comprehensive number types", () => {
    // Test i32 range
    Rong.storage.set("i32_min", -2147483648); // i32::MIN
    Rong.storage.set("i32_max", 2147483647); // i32::MAX
    Rong.storage.set("i32_regular", 42);

    // Test u32 range
    Rong.storage.set("u32_max", 4294967295); // u32::MAX
    Rong.storage.set("u32_regular", 100);

    // Test floating point
    Rong.storage.set("float_pi", 3.14159);
    Rong.storage.set("float_negative", -2.718);

    // Test large integers (should become BigInt)
    Rong.storage.set("i64_large", 9007199254740992n); // 2^53, beyond JS safe integer
    Rong.storage.set("u64_moderate", 9007199254740993n); // Just beyond JS safe integer for u64
    Rong.storage.set("i64_min", -9223372036854775808n); // i64::MIN

    // Verify all values
    assert.equal(Rong.storage.get("i32_min"), -2147483648);
    assert.equal(Rong.storage.get("i32_max"), 2147483647);
    assert.equal(Rong.storage.get("i32_regular"), 42);
    assert.equal(Rong.storage.get("u32_max"), 4294967295);
    assert.equal(Rong.storage.get("u32_regular"), 100);
    assert.equal(Rong.storage.get("float_pi"), 3.14159);
    assert.equal(Rong.storage.get("float_negative"), -2.718);
    assert.equal(Rong.storage.get("i64_large"), 9007199254740992n);
    assert.equal(Rong.storage.get("u64_moderate"), 9007199254740993n);
    assert.equal(Rong.storage.get("i64_min"), -9223372036854775808n);
  });

  it("should set and get boolean values", () => {
    Rong.storage.set("test_true", true);
    Rong.storage.set("test_false", false);

    assert.equal(Rong.storage.get("test_true"), true);
    assert.equal(Rong.storage.get("test_false"), false);
  });

  it("should handle null values", () => {
    Rong.storage.set("test_null", null);
    const value = Rong.storage.get("test_null");
    assert.equal(value, null);
  });

  it("should handle object values", () => {
    const testObj = {
      name: "test",
      value: 42,
      nested: {
        array: [1, 2, 3],
        bool: true,
        float: 3.14,
      },
      nullValue: null,
    };

    Rong.storage.set("test_object", testObj);
    const retrieved = Rong.storage.get("test_object");

    // Verify object structure
    assert.equal(retrieved.name, "test");
    assert.equal(retrieved.value, 42);
    assert.equal(retrieved.nested.bool, true);
    assert.equal(retrieved.nested.float, 3.14);
    assert.equal(retrieved.nested.array.length, 3);
    assert.equal(retrieved.nested.array[0], 1);
    assert.equal(retrieved.nested.array[1], 2);
    assert.equal(retrieved.nested.array[2], 3);
    assert.equal(retrieved.nullValue, null);
  });

  it("should handle array values", () => {
    const testArray = [
      1,
      "hello",
      true,
      null,
      { key: "value" },
      [1, 2, 3],
      3.14159,
    ];

    Rong.storage.set("test_array", testArray);
    const retrieved = Rong.storage.get("test_array");

    assert.equal(retrieved.length, 7);
    assert.equal(retrieved[0], 1);
    assert.equal(retrieved[1], "hello");
    assert.equal(retrieved[2], true);
    assert.equal(retrieved[3], null);
    assert.equal(retrieved[4].key, "value");
    assert.equal(retrieved[5].length, 3);
    assert.equal(retrieved[5][0], 1);
    assert.equal(retrieved[6], 3.14159);
  });

  it("should return undefined for non-existent keys", () => {
    const value = Rong.storage.get("non_existent_key");
    assert.equal(value, undefined);
  });

  it("should delete values", () => {
    Rong.storage.set("test_delete", "to be deleted");
    assert.equal(Rong.storage.get("test_delete"), "to be deleted");

    Rong.storage.delete("test_delete");
    assert.equal(Rong.storage.get("test_delete"), undefined);
  });

  it("should list all keys with for...of", () => {
    Rong.storage.set("key1", "value1");
    Rong.storage.set("key2", "value2");
    Rong.storage.set("key3", "value3");

    const keys = [];
    for (const key of Rong.storage.list()) {
      keys.push(key);
    }

    assert.equal(keys.length, 3);
    assert(keys.includes("key1"));
    assert(keys.includes("key2"));
    assert(keys.includes("key3"));
  });

  it("should list keys with prefix", () => {
    Rong.storage.set("user:1", "alice");
    Rong.storage.set("user:2", "bob");
    Rong.storage.set("config:theme", "dark");

    const userKeys = Array.from(Rong.storage.list("user:"));
    assert.equal(userKeys.length, 2);
    assert(userKeys.includes("user:1"));
    assert(userKeys.includes("user:2"));
    assert(!userKeys.includes("config:theme"));
  });

  it("should clear all data", () => {
    Rong.storage.set("key1", "value1");
    Rong.storage.set("key2", { test: true });

    const keysBefore = Array.from(Rong.storage.list());
    assert.equal(keysBefore.length, 2);

    Rong.storage.clear();

    const keysAfter = Array.from(Rong.storage.list());
    assert.equal(keysAfter.length, 0);
  });

  it("should provide storage info", () => {
    Rong.storage.set("test", { some: "data", with: ["nested", "content"] });

    const info = Rong.storage.info();
    assert(typeof info.currentSize === "number");
    assert(typeof info.limitSize === "number");
    assert(info.currentSize > 0);
    assert(info.limitSize > 0);
  });

  it("should reject undefined values", () => {
    let errorThrown = false;
    try {
      Rong.storage.set("test_undefined", undefined);
    } catch (e) {
      errorThrown = true;
      assert(e.message.includes("Cannot store undefined values"));
    }
    assert(errorThrown, "Should throw error for undefined values");
  });
});
