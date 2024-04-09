/*
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
*/

describe("DOMException()", () => {
  const e = new DOMException();

  it("DOMException() should have a message", () => {
    expect(e.message).toBeDefined();
  });
  it("message property should be the initial value", () => {
    expect(e.message).toEqual("");
  });
  it("DOMException() should have a name", () => {
    expect(e.name).toBeDefined();
  });
  it("name property should be the initial value", () => {
    expect(e.name).toEqual("Error");
  });
  it("DOMException() should have a stack", () => {
    expect(e.stack).toBeDefined();
  });
  it("DOMException() should have a toString()", () => {
    expect(e.toString()).toBeDefined();
  });
  it("DOMException() should be the string 'Error'", () => {
    expect(e.toString()).toEqual("Error");
  });
  it("Message properties are the same for thrown and caught exceptions", () => {
    try {
      throw new DOMException();
    } catch (ex) {
      expect(ex.message).toEqual("");
    }
  });
  it("Name properties are the same for thrown and caught exceptions", () => {
    try {
      throw new DOMException();
    } catch (ex) {
      expect(ex.name).toEqual("Error");
    }
  });
});

describe("DOMException('abc')", () => {
  const e = new DOMException("abc");

  it("message property should be the string 'abc'", () => {
    expect(e.message).toEqual("abc");
  });
  it("name property should be the initial value", () => {
    expect(e.name).toEqual("Error");
  });
  it("DOMException().toString() should be the string 'Error: abc'", () => {
    expect(e.toString()).toEqual("Error: abc");
  });
  it("Message properties are the same for thrown and caught exceptions", () => {
    try {
      throw new DOMException("abc");
    } catch (ex) {
      expect(ex.message).toEqual("abc");
    }
  });
  it("Name properties are the same for thrown and caught exceptions", () => {
    try {
      throw new DOMException("abc");
    } catch (ex) {
      expect(ex.name).toEqual("Error");
    }
  });
});

describe("DOMException('abc', 'def')", () => {
  const e = new DOMException("abc", "def");

  it("message property should be the string 'abc'", () => {
    expect(e.message).toEqual("abc");
  });
  it("name property should be the string 'def'", () => {
    expect(e.name).toEqual("def");
  });
  it("DOMException().toString() should be the string 'def: abc'", () => {
    expect(e.toString()).toEqual("def: abc");
  });
  it("Message properties are the same for thrown and caught exceptions", () => {
    try {
      throw new DOMException("abc", "def");
    } catch (ex) {
      expect(ex.message).toEqual("abc");
    }
  });
  it("Name properties are the same for thrown and caught exceptions", () => {
    try {
      throw new DOMException("abc", "def");
    } catch (ex) {
      expect(ex.name).toEqual("def");
    }
  });
});
