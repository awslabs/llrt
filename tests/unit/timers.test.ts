import timers from "timers";

describe("timers", () => {
  it("should set timeout", async () => {
    const start = Date.now();
    await new Promise((resolve) => {
      setTimeout(resolve, 10);
    });
    const end = Date.now();
    expect(end - start >= 10).toBeTruthy();
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

    expect(end - start >= 10).toBeTruthy();
    expect(status).toEqual("cleared")
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
    expect(end - start >= 10).toBeTruthy();
    expect(count).toEqual(5)
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
    expect(end - start > 10).toBeTruthy();
    expect(count).toEqual(2)
  });

  it("should import timers", () => {
    expect(timers.setTimeout).toEqual(setTimeout)
  });

  it("delay is optional", async () => {
    const start = Date.now();
    await new Promise((resolve) => {
      setTimeout(resolve);
    });
    const end = Date.now();
    expect(end - start >= 0).toBeTruthy();
  });

  it("delay can be negative.", async () => {
    const start = Date.now();
    await new Promise((resolve) => {
      setTimeout(resolve, -1);
    });
    const end = Date.now();
    expect(end - start >= 0).toBeTruthy();
  });
});
