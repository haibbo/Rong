describe("ReadableStream (constructor + reader)", () => {
  it("new ReadableStream(start) enqueues and closes", async () => {
    const rs = new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode("a"));
        controller.enqueue(new TextEncoder().encode("b"));
        controller.close();
      },
    });

    const reader = rs.getReader();
    const r1 = await reader.read();
    expect(r1.done).toBe(false);
    expect(new TextDecoder().decode(r1.value)).toBe("a");

    const r2 = await reader.read();
    expect(r2.done).toBe(false);
    expect(new TextDecoder().decode(r2.value)).toBe("b");

    const r3 = await reader.read();
    expect(r3.done).toBe(true);
  });

  it("getReader locking + releaseLock", async () => {
    const rs = new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode("x"));
        controller.close();
      },
    });

    const r1 = rs.getReader();
    let threw = false;
    try {
      rs.getReader();
    } catch (e) {
      threw = true;
      expect(e instanceof TypeError).toBe(true);
    }
    if (!threw) throw new Error("expected getReader to throw while locked");

    await r1.releaseLock();
    const r2 = rs.getReader();
    const { value, done } = await r2.read();
    expect(done).toBe(false);
    expect(new TextDecoder().decode(value)).toBe("x");
  });
});

describe("WritableStream (sink + writer)", () => {
  it("collects chunks via sink.write and close", async () => {
    const collected = [];
    const ws = new WritableStream({
      write(chunk) {
        const u8 = new Uint8Array(chunk);
        collected.push(...u8);
      },
      close() {
        // no-op
      },
    });

    const writer = ws.getWriter();
    const enc = new TextEncoder();
    await writer.write(enc.encode("hello"));
    await writer.write(enc.encode("world"));
    await writer.close();

    const text = new TextDecoder().decode(new Uint8Array(collected));
    expect(text).toBe("helloworld");
  });
});
