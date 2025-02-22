describe("AbortSignal", () => {
  describe("Basic functionality", () => {
    it("should throw error when trying to create AbortSignal with new", () => {
      try {
        new AbortSignal();
      } catch (error) {
        expect(error.message).toContain("Illegal constructor");
      }
    });

    it("should create a new AbortSignal instance via AbortController", () => {
      const controller = new AbortController();
      const signal = controller.signal;
      expect(signal.aborted).toBeFalsy();
      expect(signal.reason).toBeUndefined();
    });

    it("should handle abort event", (done) => {
      const controller = new AbortController();
      const signal = controller.signal;
      signal.onabort = () => {
        expect(signal.aborted).toBeTruthy();
        done();
      };
      controller.abort();
    });
  });

  describe("Static methods", () => {
    it("should create aborted signal with abort()", () => {
      const reason = new Error("Aborted");
      const signal = AbortSignal.abort(reason);
      expect(signal.aborted).toBeTruthy();
      expect(signal.reason).toBe(reason);
    });

    it("should create signal from multiple signals with any()", () => {
      const controller1 = new AbortController();
      const controller2 = new AbortController();
      const controller3 = new AbortController();

      const signal1 = controller1.signal;
      const signal2 = controller2.signal;
      const signal3 = controller3.signal;

      const combinedSignal = AbortSignal.any([signal1, signal2, signal3]);
      expect(combinedSignal.aborted).toBeFalsy();

      const reason = new Error("Test abort");
      controller2.abort(reason);

      expect(combinedSignal.aborted).toBeTruthy();
      expect(combinedSignal.reason).toBe(reason);
    });

    it("should abort immediately if any input signal is already aborted", () => {
      const controller1 = new AbortController();
      const signal2 = AbortSignal.abort("Already aborted");
      const controller3 = new AbortController();

      const combinedSignal = AbortSignal.any([
        controller1.signal,
        signal2,
        controller3.signal,
      ]);
      expect(combinedSignal.aborted).toBeTruthy();
      expect(combinedSignal.reason).toBe("Already aborted");
    });

    describe("timeout", () => {
      it("should create an AbortSignal that aborts after the specified time", async () => {
        const timeoutDuration = 100; // 100ms
        const signal = AbortSignal.timeout(timeoutDuration);

        // Wait for the signal to be aborted
        await new Promise((resolve, reject) => {
          signal.addEventListener("abort", () => {
            try {
              console.log("Abort event triggered");
              expect(signal.aborted).toBe(true);
              expect(signal.reason).toBeInstanceOf(DOMException);
              resolve();
            } catch (e) {
              reject(e);
            }
          });

          // Ensure the signal is not aborted before the timeout
          setTimeout(() => {
            try {
              expect(signal.aborted).toBe(false);
            } catch (e) {
              reject(e);
            }
          }, timeoutDuration / 2);

          setTimeout(() => {
            reject(new Error("Signal did not abort within expected time"));
          }, timeoutDuration * 2);
        });
      });
    });
  });

  describe("Error handling", () => {
    it("should throw if aborted", () => {
      const signal = AbortSignal.abort("Test reason");
      expect(() => signal.throwIfAborted()).toThrow("Test reason");
    });

    it("should not throw if not aborted", () => {
      const controller = new AbortController();
      const signal = controller.signal;
      assert.doesNotThrow(() => signal.throwIfAborted());
    });
  });
});
