import assert from "assert";

describe("assert.ok", () => {
  it("Should be returned 'undefined' (So it's not an error)", () => {
    expect(assert.ok(true)).toBeUndefined(); //bool
    expect(assert.ok(1)).toBeUndefined(); // numeric
    expect(assert.ok("non-empty string")).toBeUndefined(); // string
    expect(assert.ok([])).toBeUndefined(); // array
    expect(assert.ok({})).toBeUndefined(); // object
    expect(assert.ok(() => {})).toBeUndefined(); // function
    // expect(assert.ok(123n)).toBeUndefined(); // bigint
    expect(assert.ok(Symbol())).toBeUndefined(); // symbol
    expect(assert.ok(new Error())).toBeUndefined(); // error
    class AssertTestClass {}
    expect(assert.ok(AssertTestClass)).toBeUndefined(); // constructor
  });

  it("Should be returned exception", () => {
    const errMsg =
      "AssertionError: The expression was evaluated to a falsy value";
    expect(() => assert.ok(false)).toThrow(errMsg);
    expect(() => assert.ok(0)).toThrow(errMsg);
    expect(() => assert.ok("")).toThrow(errMsg);
    expect(() => assert.ok(null)).toThrow(errMsg);
  });

  it("should be returned as original error message", () => {
    const errMsg = "Error: Value must be true";
    expect(() => assert.ok(false, errMsg)).toThrow(errMsg);
  });

  it("should be returned as original error", () => {
    const errMsg = "Error: This is error";
    expect(() => assert.ok(false, Error(errMsg))).toThrow(errMsg);
  });

  it("Should be handled correctly even within functions", () => {
    const errMsg = "Error: Value should be truthy";
    function checkValue(value) {
      assert.ok(value, errMsg);
    }
    expect(checkValue(true)).toBeUndefined();
    expect(() => checkValue(false)).toThrow(errMsg);
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
