describe("EventEmitter", () => {
  let emitter;

  beforeEach(() => {
    emitter = new EventEmitter();
  });

  describe("Basic event listening and emitting", () => {
    it("should add and trigger event listeners", (done) => {
      const data = { message: "hello" };
      emitter.on("test", (arg) => {
        expect(arg).toEqual(data);
        done();
      });
      emitter.emit("test", data);
    });

    it("should handle Symbol event keys", () => {
      const sym = Symbol("test");
      let received = false;
      emitter.on(sym, () => (received = true));
      emitter.emit(sym);
      expect(received).toBeTruthy();
    });

    it("should remove Symbol event listeners", () => {
      const sym = Symbol("test");
      let count = 0;
      const listener = () => count++;
      emitter.on(sym, listener);
      emitter.emit(sym);
      emitter.removeListener(sym, listener);
      emitter.emit(sym);
      expect(count).toBe(1);
    });

    it("should call multiple listeners in registration order", () => {
      const calls = [];
      emitter.on("test", () => calls.push(1));
      emitter.on("test", () => calls.push(2));
      emitter.emit("test");

      expect(calls.length).toBe(2);
      expect(calls[0]).toBe(1);
      expect(calls[1]).toBe(2);
    });

    it("should emit events with multiple arguments", () => {
      let receivedArgs;
      emitter.on("test", (arg1, arg2, arg3) => {
        receivedArgs = [arg1, arg2, arg3];
      });
      emitter.emit("test", 1, "two", { three: 3 });
      expect(receivedArgs[0]).toBe(1);
      expect(receivedArgs[1]).toBe("two");
      expect(receivedArgs[2].three).toBe(3);
    });
  });

  describe("Special listener methods", () => {
    it("should trigger once listener only once", () => {
      let count = 0;
      emitter.once("test", () => count++);
      emitter.emit("test");
      emitter.emit("test");
      expect(count).toEqual(1);
    });

    it("should prepend listener to the beginning", () => {
      const calls = [];
      emitter.on("test", () => calls.push(2));
      emitter.prependListener("test", () => calls.push(1));
      emitter.emit("test");

      expect(calls.length).toBe(2);
      expect(calls[0]).toBe(1);
      expect(calls[1]).toBe(2);
    });

    it("should prepend once listener and remove it after execution", () => {
      let callCount = 0;
      emitter.prependOnceListener("test", () => callCount++);
      emitter.emit("test");
      emitter.emit("test");
      expect(callCount).toEqual(1);
    });
  });

  describe("Listener management", () => {
    it("should remove specific listener", () => {
      let count = 0;
      const listener = () => count++;
      emitter.on("test", listener);
      emitter.emit("test");
      emitter.removeListener("test", listener);
      emitter.emit("test");
      expect(count).toEqual(1);
    });

    it("should remove all listeners for a specific event", () => {
      let callCount = 0;
      const listener = () => callCount++;
      emitter.on("test1", listener);
      emitter.on("test2", listener);
      emitter.removeAllListeners("test1");
      emitter.emit("test1");
      emitter.emit("test2");
      expect(callCount).toEqual(1);
    });

    it("should remove all listeners for all events", () => {
      let callCount1 = 0;
      let callCount2 = 0;
      const listener1 = () => callCount1++;
      const listener2 = () => callCount2++;
      emitter.on("test1", listener1);
      emitter.on("test2", listener2);
      emitter.removeAllListeners();
      emitter.emit("test1");
      emitter.emit("test2");
      expect(callCount1).toEqual(0);
      expect(callCount2).toEqual(0);
    });
  });

  describe("Event emission", () => {
    it("should return false when emitting event with no listeners", () => {
      const result = emitter.emit("no_listeners");
      expect(result).toBe(false);
    });

    it("should return true when emitting event with listeners", () => {
      emitter.on("test", () => {});
      const result = emitter.emit("test");
      expect(result).toBe(true);
    });
  });

  describe("Edge cases", () => {
    it("should handle emitting events with no listeners", () => {
      expect(emitter.emit("no_listeners")).toBe(false);
    });

    it("should handle removing non-existent listeners", () => {
      const listener = () => {};
      let errorThrown = false;
      try {
        emitter.removeListener("test", listener);
      } catch (e) {
        errorThrown = true;
      }
      expect(errorThrown).toBeFalsy();
    });

    it("should allow removing a listener during emit", () => {
      const payloads = [];

      function handlePing(payload) {
        payloads.push(payload.count);
        emitter.off("ping", handlePing);
      }

      emitter.on("ping", handlePing);

      expect(emitter.emit("ping", { count: 1 })).toBe(true);
      expect(emitter.emit("ping", { count: 2 })).toBe(false);

      expect(payloads).toEqual([1]);
    });

    it("should handle async listeners", async () => {
      let resolved = false;
      emitter.on("test", async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
        resolved = true;
      });
      emitter.emit("test");
      await new Promise((resolve) => setTimeout(resolve, 20));
      expect(resolved).toBeTruthy();
    });
  });

  describe("Max listeners", () => {
    it("should set and get max listeners", () => {
      emitter.setMaxListeners(20);
      expect(emitter.getMaxListeners()).toBe(20);
    });

    it("should warn when exceeding max listeners", () => {
      emitter.setMaxListeners(1);
      emitter.on("test1", () => {});
      let error = null;
      try {
        emitter.on("test1", () => {});
      } catch (e) {
        error = e;
      }
      expect(error.message).toContain("EventEmitter overflow");
      expect(error.message).toContain("Use setMaxListeners()");
    });
  });

  describe("Event names", () => {
    it("should return all event names", () => {
      emitter.on("test1", () => {});
      emitter.on("test2", () => {});
      const eventNames = emitter.eventNames();
      expect(eventNames).toContain("test1");
      expect(eventNames).toContain("test2");
    });
  });

  describe("Prepend listeners", () => {
    it("should prepend listener to the beginning", () => {
      const calls = [];
      emitter.on("test", () => calls.push(2));
      emitter.prependListener("test", () => calls.push(1));
      emitter.emit("test");

      expect(calls.length).toBe(2);
      expect(calls[0]).toBe(1);
      expect(calls[1]).toBe(2);
    });

    it("should prepend once listener and remove it after execution", () => {
      let callCount = 0;
      emitter.prependOnceListener("test", () => callCount++);
      emitter.emit("test");
      emitter.emit("test");
      expect(callCount).toEqual(1);
    });
  });

  describe("listenerCount", () => {
    it("should return 0 when no listeners exist", () => {
      const emitter = new EventEmitter();
      expect(emitter.listenerCount("test")).toBe(0);
    });

    it("should return correct count for event listeners", () => {
      const emitter = new EventEmitter();
      const fn1 = () => {};
      const fn2 = () => {};

      emitter.on("test", fn1);
      emitter.on("test", fn2);

      expect(emitter.listenerCount("test")).toBe(2);
    });

    it("should return 1 when checking specific listener", () => {
      const emitter = new EventEmitter();
      const fn1 = () => {};
      const fn2 = () => {};

      emitter.on("test", fn1);
      emitter.on("test", fn2);

      expect(emitter.listenerCount("test", fn1)).toBe(1);
    });

    it("should return 0 when checking non-existent listener", () => {
      const emitter = new EventEmitter();
      const fn1 = () => {};
      const fn2 = () => {};

      emitter.on("test", fn1);

      expect(emitter.listenerCount("test", fn2)).toBe(0);
    });

    it("should return correct count after removing listeners", () => {
      const emitter = new EventEmitter();
      const fn1 = () => {};
      const fn2 = () => {};

      emitter.on("test", fn1);
      emitter.on("test", fn2);
      emitter.off("test", fn1);

      expect(emitter.listenerCount("test")).toBe(1);
    });
  });
});

describe("Event", () => {
  it("should create an Event instance with correct properties", () => {
    const event = new Event("test", { bubbles: true, cancelable: true });
    expect(event.type).toEqual("test");
    expect(event.bubbles).toBeTruthy();
    expect(event.cancelable).toBeTruthy();
  });
});

describe("CustomEvent", () => {
  it("should create a CustomEvent instance with correct properties and detail", () => {
    const customEvent = new CustomEvent("test", { detail: { key: "value" } });
    expect(customEvent.type).toEqual("test");
    expect(customEvent.detail.key).toEqual("value");
  });
});

describe("EventTarget", () => {
  it("should dispatch events to listeners", () => {
    const target = new EventTarget();
    let callCount = 0;
    const listener = () => callCount++;
    target.addEventListener("test", listener);
    target.dispatchEvent(new Event("test"));
    expect(callCount).toEqual(1);
  });

  it("should not dispatch events after removing listeners", () => {
    const target = new EventTarget();
    let callCount = 0;
    const listener = () => callCount++;
    target.addEventListener("test", listener);
    target.removeEventListener("test", listener);
    target.dispatchEvent(new Event("test"));
    expect(callCount).toEqual(0);
  });
});
