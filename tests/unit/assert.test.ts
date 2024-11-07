import * as assert from "assert";

describe("assert.ok", () => {
  it("Should be returned 'undefined' (So it's not an error)", () => {
    expect(assert.ok(true)).toBeUndefined();
    expect(assert.ok(1)).toBeUndefined();
    expect(assert.ok("non-empty string")).toBeUndefined();
    expect(assert.ok([])).toBeUndefined();
    expect(assert.ok({})).toBeUndefined();
  });

  it("Should be returned exception", () => {
    const errMsg = "The expression was evaluated to a falsy value";
    try {
      assert.ok(false);
    } catch (err) {
      expect(err.message).toEqual(errMsg);
    }
    try {
      assert.ok(0);
    } catch (err) {
      expect(err.message).toEqual(errMsg);
    }
    try {
      assert.ok("");
    } catch (err) {
      expect(err.message).toEqual(errMsg);
    }
    try {
      assert.ok(null);
    } catch (err) {
      expect(err.message).toEqual(errMsg);
    }
  });

  it("should be returned as original error message", () => {
    try {
      assert.ok(false, "Value must be true");
    } catch (err) {
      expect(err.message).toEqual("Value must be true");
    }
  });

  it("should be returned as original error", () => {
    try {
      assert.ok(false, Error("This is error"));
    } catch (err) {
      expect(err.message).toEqual("This is error");
    }
  });

  it("Should be handled correctly even within functions", () => {
    function checkValue(value) {
      assert.ok(value, "Value should be truthy");
    }
    expect(checkValue(true)).toBeUndefined();
    try {
      checkValue(false);
    } catch (err) {
      expect(err.message).toEqual("Value should be truthy");
    }
  });
});
