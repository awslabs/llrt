describe("globalThis", () => {
  it("globalThis should have a DOMException", () => {
    expect(globalThis.DOMException()).toBeDefined();
  });
  it("globalThis.DOMException() should have a message", () => {
    expect(globalThis.DOMException().message).toBeDefined();
  });
  it("globalThis.DOMException() should have a name", () => {
    expect(globalThis.DOMException().name).toBeDefined();
  });
  it("globalThis.DOMException() should have a stack", () => {
    expect(globalThis.DOMException().stack).toBeDefined();
  });
  it("globalThis.DOMException() should have a toString()", () => {
    expect(globalThis.DOMException().toString()).toBeDefined();
  });
});

describe("DOMException()", () => {
  const e = new DOMException();

  it("should have a message", () => {
    expect(e.message).toBeDefined();
  });
  it("message should be the initial value", () => {
    expect(e.message).toEqual("");
  });
  it("should have a name", () => {
    expect(e.name).toBeDefined();
  });
  it("name should be the initial value", () => {
    expect(e.name).toEqual("Error");
  });
  it("should have a stack", () => {
    expect(e.stack).toBeDefined();
  });
  it("should have a toString()", () => {
    expect(e.toString()).toBeDefined();
  });
  it("toString() should return the string 'Error'", () => {
    expect(e.toString()).toEqual("Error");
  });
  it("message should be the same for thrown and caught exceptions", () => {
    try {
      throw new DOMException();
    } catch (ex) {
      expect(ex.message).toEqual("");
    }
  });
  it("name should be the same for thrown and caught exceptions", () => {
    try {
      throw new DOMException();
    } catch (ex) {
      expect(ex.name).toEqual("Error");
    }
  });
});

describe("DOMException('abc')", () => {
  const e = new DOMException("abc");

  it("message should be the string 'abc'", () => {
    expect(e.message).toEqual("abc");
  });
  it("name should be the initial value", () => {
    expect(e.name).toEqual("Error");
  });
  it("toString() should return the string 'Error: abc'", () => {
    expect(e.toString()).toEqual("Error: abc");
  });
  it("message should be the same for thrown and caught exceptions", () => {
    try {
      throw new DOMException("abc");
    } catch (ex) {
      expect(ex.message).toEqual("abc");
    }
  });
  it("name should be the same for thrown and caught exceptions", () => {
    try {
      throw new DOMException("abc");
    } catch (ex) {
      expect(ex.name).toEqual("Error");
    }
  });
});

describe("DOMException('abc', 'def')", () => {
  const e = new DOMException("abc", "def");

  it("message should be the string 'abc'", () => {
    expect(e.message).toEqual("abc");
  });
  it("name should be the string 'def'", () => {
    expect(e.name).toEqual("def");
  });
  it("toString() should return the string 'def: abc'", () => {
    expect(e.toString()).toEqual("def: abc");
  });
  it("message should be the same for thrown and caught exceptions", () => {
    try {
      throw new DOMException("abc", "def");
    } catch (ex) {
      expect(ex.message).toEqual("abc");
    }
  });
  it("name should be the same for thrown and caught exceptions", () => {
    try {
      throw new DOMException("abc", "def");
    } catch (ex) {
      expect(ex.name).toEqual("def");
    }
  });
});
