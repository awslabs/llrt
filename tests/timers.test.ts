import timers from "timers";

describe("timers", () => {
  it("should set timeout", async () => {
    const start = Date.now();
    await new Promise((resolve) => {
      setTimeout(resolve, 10);
    });
    const end = Date.now();
    assert.ok(end - start >= 10);
  });

  it("should set nested timeout", (done) => {
    setTimeout(() => {
      setTimeout(done, 10);
    }, 10);
  });

  it("should clear timeout", async () => {
    const start = Date.now();
    let status = "";
    await new Promise<void>((resolve) => {
      let timeout = setTimeout(() => {
        status = "not-cleared";
        resolve();
      }, 5);

      setTimeout(() => {
        status = "cleared";
        resolve();
      }, 10);

      clearTimeout(timeout);
    });
    const end = Date.now();

    assert.ok(end - start >= 10);
    assert.equal(status, "cleared");
  });

  it("should set interval", async () => {
    const start = Date.now();
    let count = 1;
    await new Promise<void>((resolve) => {
      let interval = setInterval(() => {
        if (count > 4) {
          clearInterval(interval);
          return resolve();
        }
        count++;
      }, 5);
    });
    const end = Date.now();
    assert.ok(end - start >= 10);
    assert.equal(count, 5);
  });

  it("should clear interval", async () => {
    const start = Date.now();
    let count = 1;
    await new Promise<void>((resolve) => {
      let interval = setInterval(() => {
        if (count == 2) {
          clearInterval(interval);
          return;
        }
        count++;
      }, 5);
      setTimeout(resolve, 20);
    });
    const end = Date.now();
    assert.ok(end - start > 10);
    assert.equal(count, 2);
  });

  it("should import timers", () => {
    assert.equal(timers.setTimeout, setTimeout);
  });
});
