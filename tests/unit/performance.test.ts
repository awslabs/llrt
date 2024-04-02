import defaultImport from "performance";
import * as namedImport from "performance";

describe("performance.timeOrigin", () => {
  it("should have a performance timeOrigin", () => {
    expect(defaultImport.timeOrigin).toEqual(performance.timeOrigin);
    expect(namedImport.timeOrigin).toEqual(performance.timeOrigin);
  });
  it("should have a positive value", () => {
    expect(Number(performance.timeOrigin)).toBeGreaterThanOrEqual(0);
  });
});

describe("performance.now()", () => {
  it("should have a performance now", () => {
    expect(defaultImport.now()).toBeDefined();
    expect(namedImport.now()).toBeDefined();
  });
  it("should have a positive value", () => {
    expect(Number(performance.now())).toBeGreaterThanOrEqual(0);
  });
  it("should be a monotonic clock", () => {
    const before = performance.now();
    const after = performance.now();
    expect(Number(after)).toBeGreaterThanOrEqual(Number(before));
  });
});
