function bytesToString(bytes) {
  return new TextDecoder().decode(bytes);
}

describe("Rong Compression", () => {
  it("exposes zstd APIs on Rong", () => {
    expect(typeof Rong.zstdCompress).toBe("function");
    expect(typeof Rong.zstdCompressSync).toBe("function");
    expect(typeof Rong.zstdDecompress).toBe("function");
    expect(typeof Rong.zstdDecompressSync).toBe("function");
    expect(typeof Rong.gzip).toBe("function");
    expect(typeof Rong.gzipSync).toBe("function");
    expect(typeof Rong.gunzip).toBe("function");
    expect(typeof Rong.gunzipSync).toBe("function");
  });

  it("does not expose compression helpers on globalThis", () => {
    expect(globalThis.zstdCompress).toBe(undefined);
    expect(globalThis.zstdCompressSync).toBe(undefined);
    expect(globalThis.zstdDecompress).toBe(undefined);
    expect(globalThis.zstdDecompressSync).toBe(undefined);
    expect(globalThis.gzip).toBe(undefined);
    expect(globalThis.gzipSync).toBe(undefined);
    expect(globalThis.gunzip).toBe(undefined);
    expect(globalThis.gunzipSync).toBe(undefined);
  });

  it("round-trips data synchronously", () => {
    const input = new TextEncoder().encode(
      "rong compression sync payload rong compression sync payload",
    );
    const compressed = Rong.zstdCompressSync(input);
    expect(compressed instanceof Uint8Array).toBe(true);
    expect(compressed.byteLength > 0).toBe(true);

    const decompressed = Rong.zstdDecompressSync(compressed);
    expect(decompressed instanceof Uint8Array).toBe(true);
    expect(bytesToString(decompressed)).toBe(bytesToString(input));
  });

  it("round-trips data asynchronously", async () => {
    const input = new TextEncoder().encode(
      "rong compression async payload rong compression async payload",
    );
    const compressed = await Rong.zstdCompress(input, { level: 6 });
    expect(compressed instanceof Uint8Array).toBe(true);

    const decompressed = await Rong.zstdDecompress(compressed);
    expect(decompressed instanceof Uint8Array).toBe(true);
    expect(bytesToString(decompressed)).toBe(bytesToString(input));
  });

  it("accepts ArrayBuffer and typed array views with offsets", () => {
    const source = new Uint8Array(
      new TextEncoder().encode("__prefix__payload-with-offset__suffix__"),
    );
    const view = source.subarray(10, source.byteLength - 10);

    const compressed = Rong.zstdCompressSync(view);
    const decompressed = Rong.zstdDecompressSync(compressed);
    expect(bytesToString(decompressed)).toBe(bytesToString(view));

    const bufferCompressed = Rong.zstdCompressSync(view.buffer);
    const bufferDecompressed = Rong.zstdDecompressSync(bufferCompressed);
    expect(bytesToString(bufferDecompressed)).toBe(bytesToString(source));
  });

  it("supports empty payloads", async () => {
    const input = new Uint8Array();
    const compressed = await Rong.zstdCompress(input);
    const decompressed = await Rong.zstdDecompress(compressed);
    expect(decompressed.byteLength).toBe(0);
  });

  it("validates compression level", () => {
    expect(() => Rong.zstdCompressSync(new Uint8Array([1, 2, 3]), { level: 0 })).toThrow(
      TypeError,
    );
    expect(() => Rong.zstdCompressSync(new Uint8Array([1, 2, 3]), { level: 23 })).toThrow(
      TypeError,
    );
    expect(() => Rong.zstdCompressSync(new Uint8Array([1, 2, 3]), { level: 1.5 })).toThrow(
      TypeError,
    );
    expect(() => Rong.zstdCompressSync(new Uint8Array([1, 2, 3]), null)).toThrow(TypeError);
  });

  it("rejects invalid input types", () => {
    expect(() => Rong.zstdCompressSync("hello")).toThrow(TypeError);
    expect(() => Rong.zstdDecompressSync("hello")).toThrow(TypeError);
  });

  it("surfaces decompression errors for invalid payloads", async () => {
    let syncFailed = false;
    try {
      Rong.zstdDecompressSync(new Uint8Array([1, 2, 3, 4]));
    } catch (error) {
      syncFailed = true;
      expect(error instanceof Error).toBe(true);
    }
    expect(syncFailed).toBe(true);

    let asyncFailed = false;
    try {
      await Rong.zstdDecompress(new Uint8Array([1, 2, 3, 4]));
    } catch (error) {
      asyncFailed = true;
      expect(error instanceof Error).toBe(true);
    }
    expect(asyncFailed).toBe(true);
  });

  it("supports gzip round-trips synchronously", () => {
    const input = new TextEncoder().encode(
      "rong gzip payload rong gzip payload rong gzip payload",
    );
    const compressed = Rong.gzipSync(input);
    expect(compressed instanceof Uint8Array).toBe(true);
    expect(compressed.byteLength > 0).toBe(true);

    const decompressed = Rong.gunzipSync(compressed);
    expect(decompressed instanceof Uint8Array).toBe(true);
    expect(bytesToString(decompressed)).toBe(bytesToString(input));
  });

  it("supports gzip round-trips asynchronously", async () => {
    const input = new TextEncoder().encode(
      "rong gzip async payload rong gzip async payload rong gzip async payload",
    );
    const compressed = await Rong.gzip(input, { level: 6 });
    expect(compressed instanceof Uint8Array).toBe(true);
    expect(compressed.byteLength > 0).toBe(true);

    const decompressed = await Rong.gunzip(compressed);
    expect(decompressed instanceof Uint8Array).toBe(true);
    expect(bytesToString(decompressed)).toBe(bytesToString(input));
  });

  it("supports gzip levels and view inputs", () => {
    const bytes = new Uint8Array(
      new TextEncoder().encode("aa__gzip-view-payload__zz"),
    );
    const view = bytes.subarray(2, bytes.byteLength - 2);
    const compressed = Rong.gzipSync(view, { level: 9 });
    const decompressed = Rong.gunzipSync(compressed);
    expect(bytesToString(decompressed)).toBe(bytesToString(view));
  });

  it("validates gzip options and invalid payloads", () => {
    expect(() => Rong.gzipSync(new Uint8Array([1, 2, 3]), { level: -2 })).toThrow(TypeError);
    expect(() => Rong.gzipSync(new Uint8Array([1, 2, 3]), { level: 10 })).toThrow(TypeError);
    expect(() => Rong.gzipSync(new Uint8Array([1, 2, 3]), { level: 1.2 })).toThrow(TypeError);
    expect(() => Rong.gzipSync("hello")).toThrow(TypeError);
    expect(() => Rong.gunzipSync("hello")).toThrow(TypeError);

    let failed = false;
    try {
      Rong.gunzipSync(new Uint8Array([1, 2, 3, 4]));
    } catch (error) {
      failed = true;
      expect(error instanceof Error).toBe(true);
    }
    expect(failed).toBe(true);
  });

  it("surfaces async gzip decompression errors for invalid payloads", async () => {
    let failed = false;
    try {
      await Rong.gunzip(new Uint8Array([1, 2, 3, 4]));
    } catch (error) {
      failed = true;
      expect(error instanceof Error).toBe(true);
    }
    expect(failed).toBe(true);
  });

  it("accepts gzip ArrayBuffer inputs asynchronously", async () => {
    const input = new TextEncoder().encode("gzip-array-buffer-async");
    const compressed = await Rong.gzip(input.buffer);
    const restored = await Rong.gunzip(compressed.buffer);
    expect(bytesToString(restored)).toBe(bytesToString(input));
  });
});
