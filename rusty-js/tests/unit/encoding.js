describe("TextEncoder", () => {
  it("should encode basic string to Uint8Array", () => {
    const encoder = new TextEncoder();
    const encoded = encoder.encode("hello");
    const expected = new Uint8Array([104, 101, 108, 108, 111]);
    expect(encoded).toEqual(expected);
  });

  it("should encode Unicode characters to Uint8Array", () => {
    const encoder = new TextEncoder();
    const encoded = encoder.encode("你好");
    const expected = new Uint8Array([228, 189, 160, 229, 165, 189]);
    expect(encoded).toEqual(expected);
  });

  it("should encode empty string to empty Uint8Array", () => {
    const encoder = new TextEncoder();
    const encoded = encoder.encode("");
    const expected = new Uint8Array([]);
    expect(encoded).toEqual(expected);
  });

  it("should encode special characters (e.g., emoji) to Uint8Array", () => {
    const encoder = new TextEncoder();
    const encoded = encoder.encode("😊");
    const expected = new Uint8Array([240, 159, 152, 138]);
    expect(encoded).toEqual(expected);
  });
});

describe("TextDecoder", () => {
  it("should decode Uint8Array to basic string", () => {
    const decoder = new TextDecoder();
    const decoded = decoder.decode(new Uint8Array([104, 101, 108, 108, 111]));
    expect(decoded).toBe("hello");
  });

  it("should decode Uint8Array to Unicode characters", () => {
    const decoder = new TextDecoder();
    const decoded = decoder.decode(
      new Uint8Array([228, 189, 160, 229, 165, 189]),
    );
    expect(decoded).toBe("你好");
  });

  it("should decode empty Uint8Array to empty string", () => {
    const decoder = new TextDecoder();
    const decoded = decoder.decode(new Uint8Array([]));
    expect(decoded).toBe("");
  });

  it("should decode Uint8Array with special characters (e.g., emoji)", () => {
    const decoder = new TextDecoder();
    const decoded = decoder.decode(new Uint8Array([240, 159, 152, 138]));
    expect(decoded).toBe("😊");
  });

  it("should throw TypeError if input is not a Uint8Array", () => {
    const decoder = new TextDecoder();
    expect(() => decoder.decode(null)).toThrow(TypeError);
    expect(() => decoder.decode(undefined)).toThrow(TypeError);
    expect(() => decoder.decode("hello")).toThrow(TypeError);
  });

  it("should handle invalid UTF-8 sequences", () => {
    const decoder = new TextDecoder();
    const decoded = decoder.decode(new Uint8Array([0xc0, 0x80])); // Invalid UTF-8
    expect(decoded).toBe("\uFFFD\uFFFD"); // Replacement characters
  });
});

describe("Base64 Encoding", () => {
    it("should encode a string to base64", () => {
        const input = "Hello, World!";
        const encoded = btoa(input);
        expect(encoded).toBe("SGVsbG8sIFdvcmxkIQ==");
    });

    it("should decode a base64 string", () => {
        const input = "SGVsbG8sIFdvcmxkIQ==";
        const decoded = atob(input);
        expect(decoded).toBe("Hello, World!");
    });

    it("should handle empty string encoding", () => {
        const input = "";
        const encoded = btoa(input);
        expect(encoded).toBe("");
    });

    it("should handle empty string decoding", () => {
        const input = "";
        const decoded = atob(input);
        expect(decoded).toBe("");
    });

    it("should throw error when decoding invalid base64", () => {
        const input = "InvalidBase64!";
        expect(() => atob(input)).toThrow(/Failed to decode base64/);
    });

    it("should handle non-ASCII characters in encoding", () => {
        const input = "你好";
        const encoded = btoa(input);
        expect(encoded).toBe("5L2g5aW9");
    });

    it("should handle non-ASCII characters in decoding", () => {
        const input = "5L2g5aW9";
        const decoded = atob(input);
        expect(decoded).toBe("你好");
    });
});
