/*
MIT License

Copyright (c) 2021-Present Anthony Fu <https://github.com/antfu>
Copyright (c) 2021-Present Matias Capeletto <https://github.com/patak-dev>

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
 */

// Extracted and modified from Vitest:  https://github.com/vitest-dev/vitest/blob/7a31a1ae4223aed3adf260e63ac3b3f7fab3c9d7/test/core/test/jest-expect.test.ts

class TestError extends Error {}

describe("jest-expect", () => {
  it("basic", () => {
    expect(1).toBe(1);
    expect(null).toBeNull();
    expect(1).not.toBeNull();
    expect(null).toBeDefined();
    expect(undefined).not.toBeDefined();
    expect(undefined).toBeUndefined();
    expect(null).not.toBeUndefined();
    expect([]).toBeTruthy();
    expect(0).toBeFalsy();
    expect("Hello").toMatch(/llo/);
    expect("Hello").toMatch("llo");
    expect("Hello").toContain("llo");
    expect(["Hello"]).toContain("Hello");
    expect([{ text: "Hello" }]).toContainEqual({ text: "Hello" });
    expect([{ text: "Bye" }]).not.toContainEqual({ text: "Hello" });
    expect(1).toBeGreaterThan(0);

    expect(new Date(0)).toEqual(new Date(0));
    expect(new Date("inValId")).toEqual(new Date("inValId"));

    expect(new Error("message")).toEqual(new Error("message"));
    expect(new Error("message")).not.toEqual(new Error("different message"));

    expect(new URL("https://example.org")).toEqual(
      new URL("https://example.org")
    );
    expect(new URL("https://example.org")).not.toEqual(
      new URL("https://different-example.org")
    );
    expect(new URL("https://example.org?query=value")).toEqual(
      new URL("https://example.org?query=value")
    );
    expect(new URL("https://example.org?query=one")).not.toEqual(
      new URL("https://example.org?query=two")
    );
    expect(
      new URL(
        "https://subdomain.example.org/path?query=value#fragment-identifier"
      )
    ).toEqual(
      new URL(
        "https://subdomain.example.org/path?query=value#fragment-identifier"
      )
    );
    expect(
      new URL(
        "https://subdomain.example.org/path?query=value#fragment-identifier"
      )
    ).not.toEqual(
      new URL(
        "https://subdomain.example.org/path?query=value#different-fragment-identifier"
      )
    );
    expect(new URL("https://example.org/path")).toEqual(
      new URL("/path", "https://example.org")
    );
    expect(new URL("https://example.org/path")).not.toEqual(
      new URL("/path", "https://example.com")
    );

    expect(BigInt(1)).toBeGreaterThan(BigInt(0));
    expect(1).toBeGreaterThan(BigInt(0));
    expect(BigInt(1)).toBeGreaterThan(0);

    expect(1).toBeGreaterThanOrEqual(1);
    expect(1).toBeGreaterThanOrEqual(0);

    expect(BigInt(1)).toBeGreaterThanOrEqual(BigInt(1));
    expect(BigInt(1)).toBeGreaterThanOrEqual(BigInt(0));
    expect(BigInt(1)).toBeGreaterThanOrEqual(1);
    expect(1).toBeGreaterThanOrEqual(BigInt(1));

    expect(0).toBeLessThan(1);
    expect(BigInt(0)).toBeLessThan(BigInt(1));
    expect(BigInt(0)).toBeLessThan(1);

    expect(1).toBeLessThanOrEqual(1);
    expect(0).toBeLessThanOrEqual(1);
    expect(BigInt(1)).toBeLessThanOrEqual(BigInt(1));
    expect(BigInt(0)).toBeLessThanOrEqual(BigInt(1));
    expect(BigInt(1)).toBeLessThanOrEqual(1);
    expect(1).toBeLessThanOrEqual(BigInt(1));

    expect(() => {
      throw new Error("this is the error message");
    }).toThrow("this is the error message");
    expect(() => {}).not.toThrow();
    expect(() => {
      throw new TestError("error");
    }).toThrow(TestError);
    const err = new Error("hello world");
    expect(() => {
      throw err;
    }).toThrow(err);
    expect(() => {
      throw new Error("message");
    }).toThrow(
      expect.objectContaining({
        message: expect.stringContaining("mes"),
      })
    );
    expect([1, 2, 3]).toHaveLength(3);
    expect("abc").toHaveLength(3);
    expect("").not.toHaveLength(5);
    expect({ length: 3 }).toHaveLength(3);
    expect(0.2 + 0.1).not.toBe(0.3);
    expect(0.2 + 0.1).toBeCloseTo(0.3, 5);
    expect(0.2 + 0.1).not.toBeCloseTo(0.3, 100); // expect.closeTo will fail in chai
  });

  it("asymmetric matchers (jest style)", () => {
    expect({ foo: "bar" }).toEqual({ foo: expect.stringContaining("ba") });
    expect("bar").toEqual(expect.stringContaining("ba"));
    expect(["bar"]).toEqual([expect.stringContaining("ba")]);
    expect(new Set(["bar"])).toEqual(new Set([expect.stringContaining("ba")]));
    expect(new Set(["bar"])).not.toEqual(
      new Set([expect.stringContaining("zoo")])
    );

    expect({ foo: "bar" }).not.toEqual({ foo: expect.stringContaining("zoo") });
    expect("bar").not.toEqual(expect.stringContaining("zoo"));
    expect(["bar"]).not.toEqual([expect.stringContaining("zoo")]);

    expect({ foo: "bar", bar: "foo", hi: "hello" }).toEqual({
      foo: expect.stringContaining("ba"),
      bar: expect.stringContaining("fo"),
      hi: "hello",
    });
    expect(0).toEqual(expect.anything());
    expect({}).toEqual(expect.anything());
    expect("string").toEqual(expect.anything());
    expect(null).not.toEqual(expect.anything());
    expect(undefined).not.toEqual(expect.anything());
    expect({ a: 0, b: 0 }).toEqual(expect.objectContaining({ a: 0 }));
    expect({ a: 0, b: 0 }).not.toEqual(expect.objectContaining({ z: 0 }));
    expect(0).toEqual(expect.any(Number));
    expect("string").toEqual(expect.any(String));
    expect("string").not.toEqual(expect.any(Number));

    expect(["Bob", "Eve"]).toEqual(expect.arrayContaining(["Bob"]));
    expect(["Bob", "Eve"]).not.toEqual(expect.arrayContaining(["Mohammad"]));

    expect([{ name: "Bob" }, { name: "Eve" }]).toEqual(
      expect.arrayContaining<{ name: string }>([{ name: "Bob" }])
    );
    expect([{ name: "Bob" }, { name: "Eve" }]).not.toEqual(
      expect.arrayContaining<{ name: string }>([{ name: "Mohammad" }])
    );

    expect("Mohammad").toEqual(expect.stringMatching(/Moh/));
    expect("Mohammad").not.toEqual(expect.stringMatching(/jack/));
    expect({
      sum: 0.1 + 0.2,
    }).toEqual({
      sum: expect.closeTo(0.3, 5),
    });

    expect({
      sum: 0.1 + 0.2,
    }).not.toEqual({
      sum: expect.closeTo(0.4, 5),
    });

    expect({
      sum: 0.1 + 0.2,
    }).toEqual({
      // @ts-ignore
      sum: expect.not.closeTo(0.4, 5),
    });
  });

  it("asymmetric matchers negate", () => {
    expect("bar").toEqual(expect.not.stringContaining("zoo"));
    expect("bar").toEqual(expect.not.stringMatching(/zoo/));
    expect({ bar: "zoo" }).toEqual(expect.not.objectContaining({ zoo: "bar" }));
    expect(["Bob", "Eve"]).toEqual(expect.not.arrayContaining(["Steve"]));
  });

  it("object", () => {
    expect({}).toEqual({});
    expect({ apples: 13 }).toEqual({ apples: 13 });
    expect({}).toStrictEqual({});
    expect({}).not.toBe({});

    const foo = {};
    const complex = {
      "0": "zero",
      foo: 1,
      "foo.bar[0]": "baz",
      "a-b": true,
      "a-b-1.0.0": true,
      bar: {
        foo: "foo",
        bar: 100,
        arr: ["first", { zoo: "monkey" }],
      },
    };

    expect(foo).toBe(foo);
    expect(foo).toStrictEqual(foo);
    expect(complex).toMatchObject({});
    expect(complex).toMatchObject({ foo: 1 });
    expect([complex]).toMatchObject([{ foo: 1 }]);
    expect(complex).not.toMatchObject({ foo: 2 });
    expect(complex).toMatchObject({ bar: { bar: 100 } });
    expect(complex).toMatchObject({ foo: expect.any(Number) });

    expect(complex).toHaveProperty("a-b");
    expect(complex).toHaveProperty("a-b-1.0.0");
    expect(complex).toHaveProperty("0");
    expect(complex).toHaveProperty("0", "zero");
    expect(complex).toHaveProperty(["0"]);
    expect(complex).toHaveProperty(["0"], "zero");
    expect(complex).toHaveProperty([0]);
    expect(complex).toHaveProperty([0], "zero");
    expect(complex).toHaveProperty("foo");
    expect(complex).toHaveProperty("foo", 1);
    expect(complex).toHaveProperty("bar.foo", "foo");
    expect(complex).toHaveProperty("bar.arr[0]");
    expect(complex).toHaveProperty("bar.arr[1].zoo", "monkey");
    expect(complex).toHaveProperty("bar.arr.0");
    expect(complex).toHaveProperty(["bar", "arr", "0"]);
    expect(complex).toHaveProperty(["bar", "arr", "0"], "first");
    expect(complex).toHaveProperty(["bar", "arr", 0]);
    expect(complex).toHaveProperty(["bar", "arr", 0], "first");
    expect(complex).toHaveProperty("bar.arr.1.zoo", "monkey");
    expect(complex).toHaveProperty(["bar", "arr", "1", "zoo"], "monkey");
    expect(complex).toHaveProperty(["foo.bar[0]"], "baz");

    expect(complex).toHaveProperty("foo", expect.any(Number));
    expect(complex).toHaveProperty("bar", expect.any(Object));
    expect(complex).toHaveProperty("bar.arr", expect.any(Array));
    expect(complex).toHaveProperty("bar.arr.0", expect.anything());

    expect(() => {
      expect(complex).toHaveProperty("some-unknown-property");
    }).toThrowError();

    expect(() => {
      expect(complex).toHaveProperty("a-b", false);
    }).toThrowError();

    expect(() => {
      const x = { a: { b: { c: 1 } } };
      const y = { a: { b: { c: 2 } } };
      Object.freeze(x.a);
      expect(x).toEqual(y);
    }).toThrowError();
  });

  // https://jestjs.io/docs/expect#tostrictequalvalue

  class LaCroix {
    constructor(public flavor: any) {}
  }

  describe("the La Croix cans on my desk", () => {
    it("are not semantically the same", () => {
      expect(new LaCroix("lemon")).toEqual({ flavor: "lemon" });
      expect(new LaCroix("lemon")).not.toStrictEqual({ flavor: "lemon" });
    });
  });

  it("array", () => {
    expect([]).toEqual([]);
    expect([]).not.toBe([]);
    expect([]).toStrictEqual([]);

    const foo: any[] = [];

    expect(foo).toBe(foo);
    expect(foo).toStrictEqual(foo);

    const complex = [
      {
        foo: 1,
        bar: { foo: "foo", bar: 100, arr: ["first", { zoo: "monkey" }] },
      },
    ];
    expect(complex).toStrictEqual([
      {
        foo: 1,
        bar: { foo: "foo", bar: 100, arr: ["first", { zoo: "monkey" }] },
      },
    ]);
  });

  describe("toThrow", () => {
    it("error wasn't thrown", () => {
      expect(() => {
        expect(() => {}).toThrow(Error);
      }).toThrow();
    });

    it("async wasn't awaited", () => {
      expect(() => {
        expect(async () => {}).toThrow(Error);
      }).toThrow();
    });
  });
});

describe(".toStrictEqual()", () => {
  class TestClassA {
    constructor(
      public a: any,
      public b: any
    ) {}
  }

  class TestClassB {
    constructor(
      public a: any,
      public b: any
    ) {}
  }

  const TestClassC = class Child extends TestClassA {
    constructor(a: any, b: any) {
      super(a, b);
    }
  };

  const TestClassD = class Child extends TestClassB {
    constructor(a: any, b: any) {
      super(a, b);
    }
  };

  it("does not ignore keys with undefined values", () => {
    expect({
      a: undefined,
      b: 2,
    }).not.toStrictEqual({ b: 2 });
  });

  it("does not ignore keys with undefined values inside an array", () => {
    expect([{ a: undefined }]).not.toStrictEqual([{}]);
  });

  it("does not ignore keys with undefined values deep inside an object", () => {
    expect([{ a: [{ a: undefined }] }]).not.toStrictEqual([{ a: [{}] }]);
  });

  it("does not consider holes as undefined in sparse arrays", () => {
    expect([, , , 1, , ,]).not.toStrictEqual([, , , 1, undefined, ,]);
  });

  it("passes when comparing same type", () => {
    expect({
      test: new TestClassA(1, 2),
    }).toStrictEqual({ test: new TestClassA(1, 2) });
  });

  it("does not pass for different types", () => {
    expect({
      test: new TestClassA(1, 2),
    }).not.toStrictEqual({ test: new TestClassB(1, 2) });
  });

  it("does not simply compare constructor names", () => {
    const c = new TestClassC(1, 2);
    const d = new TestClassD(1, 2);
    expect(c.constructor.name).toEqual(d.constructor.name);
    expect({ test: c }).not.toStrictEqual({ test: d });
  });

  it("passes for matching sparse arrays", () => {
    expect([, 1]).toStrictEqual([, 1]);
  });

  it("does not pass when sparseness of arrays do not match", () => {
    expect([, 1]).not.toStrictEqual([undefined, 1]);
    expect([undefined, 1]).not.toStrictEqual([, 1]);
    expect([, , , 1]).not.toStrictEqual([, 1]);
  });

  it("does not pass when equally sparse arrays have different values", () => {
    expect([, 1]).not.toStrictEqual([, 2]);
  });

  it("does not pass when ArrayBuffers are not equal", () => {
    expect(Uint8Array.from([1, 2]).buffer).not.toStrictEqual(
      Uint8Array.from([0, 0]).buffer
    );
    expect(Uint8Array.from([2, 1]).buffer).not.toStrictEqual(
      Uint8Array.from([2, 2]).buffer
    );
    expect(Uint8Array.from([]).buffer).not.toStrictEqual(
      Uint8Array.from([1]).buffer
    );
  });

  it("passes for matching buffers", () => {
    expect(Uint8Array.from([1]).buffer).toStrictEqual(
      Uint8Array.from([1]).buffer
    );
    expect(Uint8Array.from([]).buffer).toStrictEqual(
      Uint8Array.from([]).buffer
    );
    expect(Uint8Array.from([9, 3]).buffer).toStrictEqual(
      Uint8Array.from([9, 3]).buffer
    );
  });

  it("does not pass for DataView", () => {
    expect(new DataView(Uint8Array.from([1, 2, 3]).buffer)).not.toStrictEqual(
      new DataView(Uint8Array.from([3, 2, 1]).buffer)
    );

    expect(new DataView(Uint16Array.from([1, 2]).buffer)).not.toStrictEqual(
      new DataView(Uint16Array.from([2, 1]).buffer)
    );
  });

  it("passes for matching DataView", () => {
    expect(new DataView(Uint8Array.from([1, 2, 3]).buffer)).toStrictEqual(
      new DataView(Uint8Array.from([1, 2, 3]).buffer)
    );
    expect(new DataView(Uint8Array.from([]).buffer)).toStrictEqual(
      new DataView(Uint8Array.from([]).buffer)
    );
  });
});

describe("toBeTypeOf()", () => {
  it("pass with typeof", () => {
    [
      [1n, "bigint"],
      [true, "boolean"],
      [false, "boolean"],
      [(() => {}) as () => void, "function"],
      [function () {} as () => void, "function"],
      [1, "number"],
      [Number.POSITIVE_INFINITY, "number"],
      [Number.NaN, "number"],
      [0, "number"],
      [{}, "object"],
      [[], "object"],
      [null, "object"],
      ["", "string"],
      ["test", "string"],
      [Symbol("test"), "symbol"],
      [undefined, "undefined"],
    ].forEach((value) => {
      // @ts-ignore
      expect(value[0]).toBeTypeOf(value[1]);
    });
  });
  it("pass with negotiation", () => {
    // @ts-ignore
    expect("test").not.toBeTypeOf("number");
  });
});

describe("toSatisfy()", () => {
  const isOdd = (value: number) => value % 2 !== 0;

  it("pass with 0", () => {
    // @ts-ignore
    expect(1).toSatisfy(isOdd);
  });

  it("pass with negotiation", () => {
    // @ts-ignore
    expect(2).not.toSatisfy(isOdd);
  });
});

it("timeout", () => new Promise((resolve) => setTimeout(resolve, 0)));
