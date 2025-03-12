describe("Blob", () => {
  // Helper function for array comparison
  async function assertArrayBufferEquals(buffer1, buffer2, message) {
    const arr1 = new Uint8Array(buffer1);
    const arr2 = new Uint8Array(buffer2);

    if (arr1.length !== arr2.length) {
      throw new Error(`${message}: Arrays have different lengths`);
    }

    for (let i = 0; i < arr1.length; i++) {
      if (arr1[i] !== arr2[i]) {
        throw new Error(`${message}: Arrays differ at index ${i}`);
      }
    }
    return true;
  }

  // Test basic Blob creation
  it("should create empty Blob", () => {
    const blob = new Blob();
    assert.equal(blob.size, 0, "Empty Blob should have size 0");
    assert.equal(blob.type, "", "Empty Blob should have empty type");
  });

  it("should create Blob with string content", () => {
    const content = "Hello, World!";
    const blob = new Blob([content]);
    assert.equal(
      blob.size,
      content.length,
      "Blob size should match content length",
    );
    assert.equal(blob.type, "", "Blob should have default empty type");
  });

  it("should create Blob with type", () => {
    const blob = new Blob(["test"], { type: "text/plain" });
    assert.equal(blob.type, "text/plain", "Blob should have correct type");
  });

  it("should normalize type to lowercase", () => {
    const blob = new Blob([], { type: "TEXT/PLAIN" });
    assert.equal(
      blob.type,
      "text/plain",
      "Type should be normalized to lowercase",
    );
  });

  it("should handle invalid type characters", () => {
    const blob = new Blob([], { type: "text/plain\0" });
    assert.equal(blob.type, "", "Invalid type should result in empty string");
  });

  // Test Blob.slice()
  it("should slice Blob correctly", () => {
    const content = "Hello, World!";
    const blob = new Blob([content]);

    const sliced = blob.slice(0, 5);
    assert.equal(sliced.size, 5, "Sliced blob should have correct size");

    const emptySlice = blob.slice(5, 1);
    assert.equal(emptySlice.size, 0, "Invalid range should create empty blob");
  });

  // Test text() method
  it("should read Blob as text", async () => {
    const content = "Hello, World!";
    const blob = new Blob([content]);

    const text = await blob.text();
    assert.equal(text, content, "text() should return correct content");
  });

  // Test arrayBuffer() method
  it("should read Blob as ArrayBuffer", async () => {
    const content = "Hello, World!";
    const blob = new Blob([content]);

    const buffer = await blob.arrayBuffer();
    const expectedArray = new TextEncoder().encode(content);
    await assertArrayBufferEquals(
      buffer,
      expectedArray.buffer,
      "arrayBuffer() should return correct data",
    );
  });

  // Test multiple parts
  it("should concatenate multiple parts", () => {
    const blob = new Blob(["Hello", " ", "World"]);
    assert.equal(blob.size, 11, "Size should be sum of all parts");
  });

  // Test different input types
  it("should handle different input types", async () => {
    const uint8Array = new Uint8Array([72, 101, 108, 108, 111]); // "Hello"
    const blob = new Blob([uint8Array, " World"]);

    const text = await blob.text();
    assert.equal(text, "Hello World", "Should handle mixed input types");
  });

  // Test error cases
  it("should handle invalid constructor arguments", () => {
    let threw = false;
    try {
      new Blob("not an array");
    } catch (e) {
      threw = true;
      assert.ok(
        e instanceof TypeError,
        "Should throw TypeError for invalid input",
      );
    }
    assert.ok(threw, "Constructor should throw for invalid input");
  });
});
