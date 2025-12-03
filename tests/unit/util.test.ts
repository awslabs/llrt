import defaultImport from "node:util";
import legacyImport from "util";

import { EventEmitter } from "node:events";

it("node:util should be the same as util", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const { inherits, inspect } = defaultImport;

describe("inherits", () => {
  it("should be inheritable parent classes", () => {
    function MyStream() {
      EventEmitter.call(this);
    }

    inherits(MyStream, EventEmitter);

    const stream = new MyStream();

    expect(stream instanceof EventEmitter).toBeTruthy();
    expect(MyStream.super_).toEqual(EventEmitter);
  });
});

describe("inspect", () => {
  it("should inspect primitive values", () => {
    expect(inspect(null)).toBe("null");
    expect(inspect(undefined)).toBe("undefined");
    expect(inspect(true)).toBe("true");
    expect(inspect(false)).toBe("false");
    expect(inspect(42)).toBe("42");
    expect(inspect(3.14)).toBe("3.14");
    expect(inspect("hello")).toBe("hello");
  });

  it("should inspect objects", () => {
    const result = inspect({ a: 1, b: 2 });
    expect(result).toContain("a:");
    expect(result).toContain("1");
    expect(result).toContain("b:");
    expect(result).toContain("2");
  });

  it("should inspect arrays", () => {
    const result = inspect([1, 2, 3]);
    expect(result).toContain("1");
    expect(result).toContain("2");
    expect(result).toContain("3");
  });

  it("should inspect nested objects with depth option", () => {
    const obj = { a: { b: { c: { d: 1 } } } };

    // Default depth is 2
    const shallow = inspect(obj, { depth: 0 });
    expect(shallow).toContain("[Object]");

    // Deep inspection
    const deep = inspect(obj, { depth: null });
    expect(deep).toContain("d:");
  });

  it("should respect maxArrayLength option", () => {
    const arr = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    const limited = inspect(arr, { maxArrayLength: 3 });
    expect(limited).toContain("1");
    expect(limited).toContain("2");
    expect(limited).toContain("3");
    expect(limited).toContain("more items");
  });

  it("should have inspect.custom symbol", () => {
    expect(inspect.custom).toBeDefined();
    expect(typeof inspect.custom).toBe("symbol");
    expect(inspect.custom).toBe(Symbol.for("nodejs.util.inspect.custom"));
  });

  it("should have inspect.defaultOptions", () => {
    expect(inspect.defaultOptions).toBeDefined();
    expect(inspect.defaultOptions.depth).toBe(2);
    expect(inspect.defaultOptions.colors).toBe(false);
    expect(inspect.defaultOptions.maxArrayLength).toBe(100);
  });

  it("should call custom inspect function", () => {
    const customSymbol = Symbol.for("nodejs.util.inspect.custom");
    const obj = {
      value: 42,
      [customSymbol]() {
        return "CustomObject<42>";
      },
    };

    const result = inspect(obj);
    expect(result).toBe("CustomObject<42>");
  });

  it("should respect customInspect: false option", () => {
    const customSymbol = Symbol.for("nodejs.util.inspect.custom");
    const obj = {
      value: 42,
      [customSymbol]() {
        return "CustomObject<42>";
      },
    };

    const result = inspect(obj, { customInspect: false });
    expect(result).not.toBe("CustomObject<42>");
    expect(result).toContain("value:");
  });

  it("should support showHidden as second argument", () => {
    const obj = { a: 1 };

    // util.inspect(object, showHidden) - legacy first arg
    const result = inspect(obj, false);
    expect(result).toContain("a:");
  });

  it("should inspect functions", () => {
    function myFunc() {}
    const result = inspect(myFunc);
    expect(result).toContain("function:");
    expect(result).toContain("myFunc");
  });

  it("should inspect Date objects", () => {
    const date = new Date("2024-01-01T00:00:00.000Z");
    const result = inspect(date);
    expect(result).toContain("2024-01-01");
  });

  it("should inspect RegExp objects", () => {
    const regex = /test/gi;
    const result = inspect(regex);
    expect(result).toContain("/test/gi");
  });

  it("should handle circular references", () => {
    const obj: any = { a: 1 };
    obj.self = obj;

    const result = inspect(obj);
    expect(result).toContain("[Circular]");
  });

  it("should respect maxStringLength option", () => {
    const longStr = "a".repeat(50);

    const truncated = inspect(longStr, { maxStringLength: 10 });
    expect(truncated).toContain("aaaaaaaaaa");
    expect(truncated).toContain("40 more characters");

    const full = inspect(longStr);
    expect(full).toBe("a".repeat(50));
  });

  it("should respect sorted option", () => {
    const obj = { zebra: 1, apple: 2, mango: 3 };

    const unsorted = inspect(obj);
    const sorted = inspect(obj, { sorted: true });

    // Sorted should have keys in alphabetical order
    expect(sorted.indexOf("apple")).toBeLessThan(sorted.indexOf("mango"));
    expect(sorted.indexOf("mango")).toBeLessThan(sorted.indexOf("zebra"));
  });

  it("should respect showHidden option", () => {
    const obj: any = {};
    Object.defineProperty(obj, "hidden", { value: "secret", enumerable: false });
    obj.visible = "public";

    const normal = inspect(obj);
    expect(normal).toContain("visible");
    expect(normal).not.toContain("hidden");

    const withHidden = inspect(obj, { showHidden: true });
    expect(withHidden).toContain("visible");
    expect(withHidden).toContain("hidden");
    expect(withHidden).toContain("secret");
  });

  it("should respect breakLength option", () => {
    const obj = { a: 1, b: 2, c: 3 };

    // With very short breakLength, should break to multiple lines
    const shortBreak = inspect(obj, { breakLength: 10 });
    // Should contain newlines for multiline output
    expect(shortBreak).toContain("\n");

    // With very long breakLength, should fit on one line
    const longBreak = inspect(obj, { breakLength: 200 });
    // Should not contain newlines (inline)
    expect(longBreak).not.toContain("\n");
  });

  it("should respect compact option", () => {
    const nested = { a: { b: { c: 1 } } };

    // compact: 0 should always use multiline
    const notCompact = inspect(nested, { compact: 0, depth: 5 });
    expect(notCompact).toContain("\n");

    // compact: 5 (high value) should try to keep things inline
    const veryCompact = inspect(nested, { compact: 5, breakLength: 200, depth: 5 });
    // With high compact and high breakLength, nested objects fit inline
    expect(veryCompact.split("\n").length).toBeLessThan(
      notCompact.split("\n").length
    );
  });

  it("should have breakLength and compact in defaultOptions", () => {
    expect(inspect.defaultOptions.breakLength).toBe(80);
    expect(inspect.defaultOptions.compact).toBe(3);
  });

  it("should support compact: false (alias for compact: 0)", () => {
    const nested = { a: { b: { c: 1 } } };

    // compact: false should behave like compact: 0 (always multiline)
    const withFalse = inspect(nested, { compact: false, depth: 5 });
    const withZero = inspect(nested, { compact: 0, depth: 5 });

    // Both should produce multiline output
    expect(withFalse).toContain("\n");
    expect(withZero).toContain("\n");

    // They should have similar structure (both multiline)
    expect(withFalse.split("\n").length).toBe(withZero.split("\n").length);
  });

  it("should support sorted with custom comparator function", () => {
    const obj = { zebra: 1, apple: 2, mango: 3 };

    // Custom comparator: reverse alphabetical order
    const reverseSort = inspect(obj, {
      sorted: (a: string, b: string) => b.localeCompare(a),
    });

    // Should have keys in reverse alphabetical order: zebra, mango, apple
    expect(reverseSort.indexOf("zebra")).toBeLessThan(reverseSort.indexOf("mango"));
    expect(reverseSort.indexOf("mango")).toBeLessThan(reverseSort.indexOf("apple"));
  });

  it("should support sorted with length-based comparator", () => {
    const obj = { ab: 1, abcd: 2, abc: 3, a: 4 };

    // Custom comparator: sort by key length
    const lengthSort = inspect(obj, {
      sorted: (a: string, b: string) => a.length - b.length,
    });

    // Should have keys sorted by length: a, ab, abc, abcd
    expect(lengthSort.indexOf("a:")).toBeLessThan(lengthSort.indexOf("ab:"));
    expect(lengthSort.indexOf("ab:")).toBeLessThan(lengthSort.indexOf("abc:"));
    expect(lengthSort.indexOf("abc:")).toBeLessThan(lengthSort.indexOf("abcd:"));
  });
});
