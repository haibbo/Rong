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

  it("supports async sink.write/close (promises)", async () => {
    const collected = [];
    const ws = new WritableStream({
      async write(chunk) {
        await new Promise((r) => setTimeout(r, 5));
        const u8 = new Uint8Array(chunk);
        collected.push(...u8);
      },
      async close() {
        await new Promise((r) => setTimeout(r, 5));
      },
    });

    const writer = ws.getWriter();
    const enc = new TextEncoder();
    await writer.write(enc.encode("foo"));
    await writer.write(enc.encode("bar"));
    await writer.close();

    const text = new TextDecoder().decode(new Uint8Array(collected));
    expect(text).toBe("foobar");
  });
});

describe("ReadableStream pipeTo", () => {
  it("pipes readable to writable (sink)", async () => {
    const enc = new TextEncoder();
    const dec = new TextDecoder();

    const rs = new ReadableStream({
      start(controller) {
        controller.enqueue(enc.encode("hello"));
        controller.enqueue(enc.encode("world"));
        controller.close();
      },
    });

    const collected = [];
    const ws = new WritableStream({
      write(chunk) {
        const u8 = new Uint8Array(chunk);
        collected.push(...u8);
      },
      close() {},
    });

    await rs.pipeTo(ws);
    const out = new TextDecoder().decode(new Uint8Array(collected));
    expect(out).toBe("helloworld");
  });

  it("respects preventClose option", async () => {
    const enc = new TextEncoder();
    let closed = false;
    const rs = new ReadableStream({
      start(controller) {
        controller.enqueue(enc.encode("x"));
        controller.close();
      },
    });
    const ws = new WritableStream({
      write(_) {},
      close() {
        closed = true;
      },
    });
    await rs.pipeTo(ws, { preventClose: true });
    expect(closed).toBe(false);
  });
});

describe("ReadableStream async iterator", () => {
  it("iterates chunks with for await...of", async () => {
    const enc = new TextEncoder();
    const rs = new ReadableStream({
      start(controller) {
        controller.enqueue(enc.encode("A"));
        controller.enqueue(enc.encode("B"));
        controller.close();
      },
    });

    const collected = [];
    for await (const chunk of rs) {
      const u8 = new Uint8Array(chunk);
      collected.push(...u8);
    }
    const out = new TextDecoder().decode(new Uint8Array(collected));
    expect(out).toBe("AB");
  });
});

describe("ReadableStream tee", () => {
  it("duplicates chunks to both branches", async () => {
    const enc = new TextEncoder();
    const dec = new TextDecoder();

    const rs = new ReadableStream({
      start(controller) {
        controller.enqueue(enc.encode("hi"));
        controller.enqueue(enc.encode("!"));
        controller.close();
      },
    });

    const [b1, b2] = rs.tee();
    expect(b1 instanceof ReadableStream).toBe(true);
    expect(b2 instanceof ReadableStream).toBe(true);

    const r1 = b1.getReader();
    const r2 = b2.getReader();

    let s1 = "";
    while (true) {
      const { done, value } = await r1.read();
      if (done) break;
      s1 += dec.decode(value);
    }

    let s2 = "";
    while (true) {
      const { done, value } = await r2.read();
      if (done) break;
      s2 += dec.decode(value);
    }

    expect(s1).toBe("hi!");
    expect(s2).toBe("hi!");
  });

  it("locks the original stream after tee", async () => {
    const enc = new TextEncoder();
    const rs = new ReadableStream({
      start(controller) {
        controller.enqueue(enc.encode("x"));
        controller.close();
      },
    });

    rs.tee();

    let threw = false;
    try {
      rs.getReader();
    } catch (e) {
      threw = true;
      expect(e instanceof TypeError).toBe(true);
    }
    if (!threw) throw new Error("expected getReader to throw after tee() (locked)");
  });
});
