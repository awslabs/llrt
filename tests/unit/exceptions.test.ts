describe("globalThis", () => {
  it("globalThis should have a DOMException", () => {
    expect(globalThis.DOMException()).toBeDefined();
  });
  it("globalThis.DOMException() should have a message", () => {
    expect(globalThis.DOMException().message).toBeDefined();
  });
  it("globalThis.DOMException() should have a message", () => {
    expect(globalThis.DOMException().name).toBeDefined();
  });
});

describe("DOMException()", () => {
  const e = new DOMException();

  it("should have a message", () => {
    expect(e.message).toBeDefined();
  });
  it("message property should be the initial value", () => {
    expect(e.message).toEqual("");
  });
  it("should have a message", () => {
    expect(e.name).toBeDefined();
  });
  it("should be the initial value", () => {
    expect(e.name).toEqual("Error");
  });
  it("result of the toString method should be 'Error'", () => {
    expect(e.toString()).toEqual("Error");
  });
  it("throwing and catching exceptions can be done", () => {
    try {
      throw new DOMException();
    } catch (ex) {
      expect(ex.toString()).toEqual(e.toString());
    }
  });
});

describe("DOMException('abc')", () => {
  const e = new DOMException("abc");

  it("message property should have the string 'abc'", () => {
    expect(e.message).toEqual("abc");
  });
  it("message property should be the initial value", () => {
    expect(e.name).toEqual("Error");
  });
  it("result of the toString method should be 'Error: abc'", () => {
    expect(e.toString()).toEqual("Error: abc");
  });
  it("throwing and catching exceptions can be done", () => {
    try {
      throw new DOMException("abc");
    } catch (ex) {
      expect(ex.toString()).toEqual(e.toString());
    }
  });
});

describe("DOMException('abc', 'def')", () => {
  const e = new DOMException("abc", "def");

  it("message property should have the string 'abc'", () => {
    expect(e.message).toEqual("abc");
  });
  it("name property should have the string 'def'", () => {
    expect(e.name).toEqual("def");
  });
  it("result of the toString method should be 'def: abc'", () => {
    expect(e.toString()).toEqual("def: abc");
  });
  it("throwing and catching exceptions can be done", () => {
    try {
      throw new DOMException("abc", "def");
    } catch (ex) {
      expect(ex.toString()).toEqual(e.toString());
    }
  });
});
