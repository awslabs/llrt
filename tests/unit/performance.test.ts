describe("performance.timeOrigin", () => {
  it("should have a positive value", () => {
    expect(Number(performance.timeOrigin)).toBeGreaterThanOrEqual(0);
  });
});

describe("performance.now()", () => {
  it("should have a positive value", () => {
    expect(Number(performance.now())).toBeGreaterThanOrEqual(0);
  });
  it("should be a monotonic clock", () => {
    const before = performance.now();
    const after = performance.now();
    expect(Number(after)).toBeGreaterThanOrEqual(Number(before));
  });

  describe("performance.toJSON()", () => {
    it("performance.toJSON().timeOrigin should have a positive value", () => {
      //@ts-ignore
      expect(Number(performance.toJSON().timeOrigin)).toBeGreaterThanOrEqual(0);
    });
    it("performance.toJSON().timeOrigin should match performance.timeOrigin", () => {
      //@ts-ignore
      expect(performance.toJSON().timeOrigin).toEqual(performance.timeOrigin);
    });
  });
});
