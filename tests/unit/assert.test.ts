import assert from "assert";

describe("assert.ok", () => {
  it("Should be returned 'undefined' (So it's not an error)", () => {
    expect(assert.ok(true)).toBeUndefined(); //bool
    expect(assert.ok(1)).toBeUndefined(); // numeric
    expect(assert.ok("non-empty string")).toBeUndefined(); // string
    expect(assert.ok([])).toBeUndefined(); // array
    expect(assert.ok({})).toBeUndefined(); // object
    expect(assert.ok(() => {})).toBeUndefined(); // function
    expect(assert.ok(123n)).toBeUndefined(); // bigint
    expect(assert.ok(Symbol())).toBeUndefined(); // symbol
    expect(assert.ok(new Error())).toBeUndefined(); // error
    class AssertTestClass {}
    expect(assert.ok(AssertTestClass)).toBeUndefined(); // constructor
  });

  it("Should be returned exception", () => {
    const errMsg =
      "AssertionError: The expression was evaluated to a falsy value";
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

describe("assert", () => {
  it("Should be returned 'undefined' (So it's not an error)", () => {
    expect(assert(true)).toBeUndefined();
    expect(assert(1)).toBeUndefined();
    expect(assert("non-empty string")).toBeUndefined();
    expect(assert([])).toBeUndefined();
    expect(assert({})).toBeUndefined();
  });
});
