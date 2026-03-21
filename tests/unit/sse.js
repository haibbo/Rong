describe("EventSource", () => {
  it("should receive SSE events via addEventListener", async () => {
    const url = TEST_SERVER_URL + "/sse/basic";
    const es = new EventSource(url);

    const events = [];
    await new Promise((resolve, reject) => {
      es.addEventListener("open", () => {
        // connection opened
      });

      es.addEventListener("message", (evt) => {
        events.push(evt);
        if (events.length === 2) {
          es.close();
          resolve();
        }
      });

      es.addEventListener("error", (evt) => {
        es.close();
        reject(new Error(evt.message || "SSE error"));
      });

      setTimeout(() => {
        es.close();
        reject(new Error("timeout"));
      }, 5000);
    });

    assert.equal(events.length, 2);
    assert.equal(events[0].type, "message");
    assert.equal(events[0].data, "hello");
    assert.equal(events[0].lastEventId, "1");
    assert.equal(events[1].data, "world");
    assert.equal(es.lastEventId, "2");
    assert.equal(es.readyState, EventSource.CLOSED);
  });

  it("should reconnect and send Last-Event-ID", async () => {
    const url = TEST_SERVER_URL + "/sse/reconnect";
    const es = new EventSource(url, {
      reconnect: {
        enabled: true,
        baseDelayMs: 10,
        maxDelayMs: 100,
      },
    });

    const ids = [];
    const payloads = [];

    await new Promise((resolve, reject) => {
      es.addEventListener("message", (evt) => {
        ids.push(evt.lastEventId);
        payloads.push(evt.data);
        if (evt.lastEventId === "2") {
          es.close();
          resolve();
        }
      });

      es.addEventListener("error", () => {
        // errors during reconnect are expected
      });

      setTimeout(() => {
        es.close();
        reject(new Error("timeout"));
      }, 5000);
    });

    assert.equal(ids.length >= 2, true);
    assert.equal(ids[0], "1");
    assert.equal(payloads[0], "first");
    assert.equal(ids.includes("2"), true);
    assert.equal(es.lastEventId, "2");
  });

  it("should apply standalone retry control frames", async () => {
    const url = TEST_SERVER_URL + "/sse/retry-control";
    const es = new EventSource(url, {
      reconnect: {
        enabled: true,
        baseDelayMs: 10,
        maxDelayMs: 500,
      },
    });
    const startedAt = Date.now();
    const ids = [];

    await new Promise((resolve, reject) => {
      es.addEventListener("message", (evt) => {
        ids.push(evt.lastEventId);
        if (evt.lastEventId === "2") {
          es.close();
          resolve();
        }
      });

      es.onerror = () => {
        // reconnect attempts may surface an error event
      };

      setTimeout(() => {
        es.close();
        reject(new Error("timeout"));
      }, 5000);
    });

    const elapsed = Date.now() - startedAt;
    assert.equal(ids[0], "1");
    assert.equal(ids.includes("2"), true);
    assert.equal(elapsed >= 180, true, `reconnect happened too quickly: ${elapsed}ms`);
  });

  it("should have correct static constants", () => {
    assert.equal(EventSource.CONNECTING, 0);
    assert.equal(EventSource.OPEN, 1);
    assert.equal(EventSource.CLOSED, 2);
  });

  it("should expose url property", () => {
    const url = TEST_SERVER_URL + "/sse/basic";
    const es = new EventSource(url);
    assert.equal(es.url, url);
    es.close();
  });

  it("should fire open event and transition readyState", async () => {
    const url = TEST_SERVER_URL + "/sse/basic";
    const es = new EventSource(url);

    // readyState starts at CONNECTING
    assert.equal(es.readyState, EventSource.CONNECTING);

    await new Promise((resolve, reject) => {
      es.addEventListener("open", (evt) => {
        assert.equal(evt.type, "open");
        assert.equal(es.readyState, EventSource.OPEN);
        es.close();
        resolve();
      });

      es.addEventListener("error", (evt) => {
        es.close();
        reject(new Error(evt.message || "SSE error"));
      });

      setTimeout(() => {
        es.close();
        reject(new Error("timeout"));
      }, 5000);
    });

    assert.equal(es.readyState, EventSource.CLOSED);
  });

  it("should not emit open before the handshake succeeds", async () => {
    const url = TEST_SERVER_URL + "/sse/not-event-stream";
    const es = new EventSource(url, { reconnect: { enabled: false } });
    let openCalled = false;

    await new Promise((resolve, reject) => {
      es.onopen = () => {
        openCalled = true;
      };

      es.onerror = (evt) => {
        assert.equal(evt.type, "error");
        setTimeout(resolve, 20);
      };

      setTimeout(() => {
        es.close();
        reject(new Error("timeout"));
      }, 5000);
    });

    assert.equal(openCalled, false);
    assert.equal(es.readyState, EventSource.CLOSED);
  });

  it("should deliver small events before stream end", async () => {
    const url = TEST_SERVER_URL + "/sse/live-small";
    const es = new EventSource(url, { reconnect: { enabled: false } });
    const startedAt = Date.now();

    await new Promise((resolve, reject) => {
      es.onmessage = (evt) => {
        const elapsed = Date.now() - startedAt;
        assert.equal(evt.data, "live-small");
        assert.equal(elapsed < 600, true, `message arrived too late: ${elapsed}ms`);
        es.close();
        resolve();
      };

      es.onerror = (evt) => {
        es.close();
        reject(new Error(evt.message || "SSE error"));
      };

      setTimeout(() => {
        es.close();
        reject(new Error("timeout"));
      }, 700);
    });
  });

  it("should support onopen and onmessage property handlers", async () => {
    const url = TEST_SERVER_URL + "/sse/basic";
    const es = new EventSource(url, { reconnect: { enabled: false } });

    let openCalled = false;
    const messages = [];

    await new Promise((resolve, reject) => {
      es.onopen = (evt) => {
        openCalled = true;
        assert.equal(evt.type, "open");
      };

      es.onmessage = (evt) => {
        messages.push(evt.data);
        if (messages.length === 2) {
          es.close();
          resolve();
        }
      };

      es.onerror = (evt) => {
        es.close();
        reject(new Error(evt.message || "SSE error"));
      };

      setTimeout(() => {
        es.close();
        reject(new Error("timeout"));
      }, 5000);
    });

    assert.equal(openCalled, true);
    assert.equal(messages.length, 2);
    assert.equal(messages[0], "hello");
    assert.equal(messages[1], "world");
  });

  it("should replace and clear onmessage handler", async () => {
    const url = TEST_SERVER_URL + "/sse/basic";
    const es = new EventSource(url, { reconnect: { enabled: false } });

    let firstCount = 0;
    let secondCount = 0;

    await new Promise((resolve, reject) => {
      es.onmessage = () => {
        firstCount++;
      };

      es.onmessage = () => {
        secondCount++;
        es.onmessage = null;
        setTimeout(() => {
          es.close();
          resolve();
        }, 50);
      };

      es.onerror = (evt) => {
        es.close();
        reject(new Error(evt.message || "SSE error"));
      };

      setTimeout(() => {
        es.close();
        reject(new Error("timeout"));
      }, 5000);
    });

    assert.equal(firstCount, 0);
    assert.equal(secondCount, 1);
    assert.equal(es.onmessage, null);
  });

  it("should keep addEventListener listeners when clearing the same property handler", async () => {
    const url = TEST_SERVER_URL + "/sse/basic";
    const es = new EventSource(url, { reconnect: { enabled: false } });
    let count = 0;

    await new Promise((resolve, reject) => {
      const shared = () => {
        count++;
        if (count === 2) {
          es.close();
          resolve();
        }
      };

      es.onmessage = shared;
      es.addEventListener("message", shared);
      es.onmessage = null;

      es.onerror = (evt) => {
        es.close();
        reject(new Error(evt.message || "SSE error"));
      };

      setTimeout(() => {
        es.close();
        reject(new Error("timeout"));
      }, 5000);
    });

    assert.equal(es.onmessage, null);
    assert.equal(count, 2);
  });

  it("should allow onmessage and addEventListener together", async () => {
    const url = TEST_SERVER_URL + "/sse/basic";
    const es = new EventSource(url, { reconnect: { enabled: false } });

    let propertyCount = 0;
    let listenerCount = 0;

    await new Promise((resolve, reject) => {
      es.onmessage = () => {
        propertyCount++;
      };

      es.addEventListener("message", () => {
        listenerCount++;
      });

      es.addEventListener("message", (evt) => {
        if (evt.lastEventId === "2") {
          es.close();
          resolve();
        }
      });

      es.onerror = (evt) => {
        es.close();
        reject(new Error(evt.message || "SSE error"));
      };

      setTimeout(() => {
        es.close();
        reject(new Error("timeout"));
      }, 5000);
    });

    assert.equal(propertyCount, 2);
    assert.equal(listenerCount, 2);
  });

  it("should dispatch custom event types", async () => {
    const url = TEST_SERVER_URL + "/sse/custom";
    const es = new EventSource(url, { reconnect: { enabled: false } });

    const statusEvents = [];
    const progressEvents = [];
    const messageEvents = [];

    await new Promise((resolve, reject) => {
      es.addEventListener("status", (evt) => {
        statusEvents.push(evt);
      });

      es.addEventListener("progress", (evt) => {
        progressEvents.push(evt);
      });

      es.addEventListener("message", (evt) => {
        messageEvents.push(evt);
        // "message" is the last event from the server
        es.close();
        resolve();
      });

      setTimeout(() => {
        es.close();
        reject(new Error("timeout"));
      }, 5000);
    });

    assert.equal(statusEvents.length, 1);
    assert.equal(statusEvents[0].type, "status");
    assert.equal(statusEvents[0].data, "connected");

    assert.equal(progressEvents.length, 1);
    assert.equal(progressEvents[0].type, "progress");
    assert.equal(progressEvents[0].data, "50%");

    assert.equal(messageEvents.length, 1);
    assert.equal(messageEvents[0].data, "default message");
    assert.equal(messageEvents[0].lastEventId, "3");
  });

  it("should stop receiving after close()", async () => {
    const url = TEST_SERVER_URL + "/sse/many";
    const es = new EventSource(url, { reconnect: { enabled: false } });

    const events = [];
    await new Promise((resolve, reject) => {
      es.addEventListener("message", (evt) => {
        events.push(evt);
        if (events.length === 2) {
          es.close();
          // Give a moment for any spurious events after close
          setTimeout(resolve, 50);
        }
      });

      setTimeout(() => {
        es.close();
        reject(new Error("timeout"));
      }, 5000);
    });

    assert.equal(es.readyState, EventSource.CLOSED);
    // Should have stopped at 2 (no more events after close)
    assert.equal(events.length, 2);
  });

  it("should support removeEventListener", async () => {
    const url = TEST_SERVER_URL + "/sse/basic";
    const es = new EventSource(url, { reconnect: { enabled: false } });

    const removed = [];
    const kept = [];

    const removedListener = (evt) => {
      removed.push(evt);
    };

    await new Promise((resolve, reject) => {
      es.addEventListener("message", removedListener);
      // Remove immediately before any events arrive
      es.removeEventListener("message", removedListener);

      es.addEventListener("message", (evt) => {
        kept.push(evt);
        if (kept.length === 2) {
          es.close();
          resolve();
        }
      });

      setTimeout(() => {
        es.close();
        reject(new Error("timeout"));
      }, 5000);
    });

    assert.equal(removed.length, 0);
    assert.equal(kept.length, 2);
  });

  it("should provide origin in event objects", async () => {
    const url = TEST_SERVER_URL + "/sse/basic";
    const es = new EventSource(url, { reconnect: { enabled: false } });

    await new Promise((resolve, reject) => {
      es.addEventListener("message", (evt) => {
        assert.equal(typeof evt.origin, "string");
        assert.equal(evt.origin.length > 0, true);
        assert.equal(evt.origin.startsWith("http://"), true);
        es.close();
        resolve();
      });

      setTimeout(() => {
        es.close();
        reject(new Error("timeout"));
      }, 5000);
    });
  });

  it("should support multiple listeners on same event", async () => {
    const url = TEST_SERVER_URL + "/sse/basic";
    const es = new EventSource(url, { reconnect: { enabled: false } });

    let callCount = 0;

    await new Promise((resolve, reject) => {
      es.addEventListener("message", () => { callCount++; });
      es.addEventListener("message", () => { callCount++; });
      es.addEventListener("message", (evt) => {
        // Third listener closes after first event
        es.close();
        resolve();
      });

      setTimeout(() => {
        es.close();
        reject(new Error("timeout"));
      }, 5000);
    });

    // Two extra listeners should each have been called once (for the first event)
    assert.equal(callCount, 2);
  });

  it("should set readyState to CLOSED when stream ends without reconnect", async () => {
    const url = TEST_SERVER_URL + "/sse/basic";
    const es = new EventSource(url, { reconnect: { enabled: false } });

    await new Promise((resolve, reject) => {
      let count = 0;
      es.addEventListener("message", () => {
        count++;
        if (count === 2) {
          // Stream has exactly 2 events, then ends
          // Wait a bit for the pump to detect end-of-stream
          setTimeout(resolve, 50);
        }
      });

      setTimeout(() => {
        es.close();
        reject(new Error("timeout"));
      }, 5000);
    });

    assert.equal(es.readyState, EventSource.CLOSED);
  });
});
