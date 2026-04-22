describe("Rong.cron", () => {
  function mustThrowTypeError(fn) {
    let threw = false;
    try {
      fn();
    } catch (error) {
      threw = error instanceof TypeError;
    }
    assert.equal(threw, true);
  }

  it("exposes cron APIs and aliases", () => {
    assert.equal(typeof Rong.cron, "function");
    assert.equal(typeof Rong.cron.parse, "function");
    assert.equal(Bun.cron, Rong.cron);
  });

  it("parses five-field cron expressions in UTC", () => {
    const start = new Date("2024-01-01T00:00:00.000Z");
    const next = Rong.cron.parse("30 9 * * MON-FRI", start);
    assert.ok(next instanceof Date);
    assert.equal(next.toISOString(), "2024-01-01T09:30:00.000Z");
  });

  it("parses from Date or epoch-millisecond relativeDate", () => {
    const start = Date.parse("2024-01-01T09:29:30.000Z");
    assert.equal(
      Rong.cron.parse("30 9 * * *", start).toISOString(),
      "2024-01-01T09:30:00.000Z",
    );
    assert.equal(
      Rong.cron.parse("30 9 * * *", new Date(start)).toISOString(),
      "2024-01-01T09:30:00.000Z",
    );
  });

  it("supports repeated parse calls for upcoming occurrences", () => {
    let cursor = new Date("2024-01-01T00:00:00.000Z");
    const times = [];
    for (let i = 0; i < 3; i++) {
      cursor = Rong.cron.parse("0 * * * *", cursor);
      times.push(cursor.toISOString());
    }
    assert.equal(times.join(","), [
      "2024-01-01T01:00:00.000Z",
      "2024-01-01T02:00:00.000Z",
      "2024-01-01T03:00:00.000Z",
    ].join(","));
  });

  it("supports nicknames and full names", () => {
    assert.equal(
      Rong.cron.parse("@daily", new Date("2024-01-01T00:00:00.000Z")).toISOString(),
      "2024-01-02T00:00:00.000Z",
    );
    assert.equal(
      Rong.cron.parse("0 9 * * Monday-Friday", new Date("2024-01-06T00:00:00.000Z")).toISOString(),
      "2024-01-08T09:00:00.000Z",
    );
    assert.equal(
      Rong.cron.parse("0 0 1 January *", new Date("2024-01-01T00:00:00.000Z")).toISOString(),
      "2025-01-01T00:00:00.000Z",
    );
  });

  it("treats day-of-month and day-of-week as OR when both are specified", () => {
    assert.equal(
      Rong.cron.parse("0 0 15 * FRI", new Date("2024-03-16T00:00:00.000Z")).toISOString(),
      "2024-03-22T00:00:00.000Z",
    );
    assert.equal(
      Rong.cron.parse("0 0 15 * FRI", new Date("2024-03-22T00:00:00.000Z")).toISOString(),
      "2024-03-29T00:00:00.000Z",
    );
  });

  it("returns null when the expression has no matching date", () => {
    assert.equal(Rong.cron.parse("0 0 30 2 *", new Date("2024-01-01T00:00:00.000Z")), null);
  });

  it("throws TypeError for invalid parse inputs", () => {
    mustThrowTypeError(() => Rong.cron.parse("*/1 * * * * *"));
    mustThrowTypeError(() => Rong.cron.parse("@reboot"));
    mustThrowTypeError(() => Rong.cron.parse("0 0 * * *", new Date("invalid")));
    mustThrowTypeError(() => Rong.cron.parse("0 0 * * *", Infinity));
  });

  it("returns a chainable in-process CronJob handle", () => {
    const job = Rong.cron("* * * * *", () => {});
    assert.equal(job.cron, "* * * * *");
    assert.equal(job.unref(), job);
    assert.equal(job.ref(), job);
    assert.equal(job.stop(), job);
  });

  it("invokes handlers with the CronJob as this and no arguments", async () => {
    let sameThis = false;
    let argc = -1;
    let calls = 0;
    const job = Rong.cron("* * * * *", function () {
      sameThis = this === job;
      argc = arguments.length;
      calls += 1;
    });

    await __triggerCronJob(job);
    job.stop();

    assert.equal(sameThis, true);
    assert.equal(argc, 0);
    assert.equal(calls, 1);
  });

  it("awaits async handlers before completing a tick", async () => {
    const events = [];
    const job = Rong.cron("* * * * *", async function () {
      events.push("start");
      await Promise.resolve();
      events.push("settled");
    });

    await __triggerCronJob(job);
    job.stop();

    assert.equal(events.join(","), "start,settled");
  });

  it("stop prevents later in-process invocations", async () => {
    let calls = 0;
    const job = Rong.cron("* * * * *", function () {
      calls += 1;
      this.stop();
    });

    await __triggerCronJob(job);
    await __triggerCronJob(job);

    assert.equal(calls, 1);
  });

  it("rejects invalid in-process cron registrations", () => {
    mustThrowTypeError(() => Rong.cron("0 0 30 2 *", () => {}));
    mustThrowTypeError(() => Rong.cron("* * * * *"));
    mustThrowTypeError(() => Rong.cron("* * * * *", "not a function"));
  });

  it("does not implement OS-level cron registration", () => {
    mustThrowTypeError(() => Rong.cron("./worker.js", "* * * * *", "job"));
  });
});
