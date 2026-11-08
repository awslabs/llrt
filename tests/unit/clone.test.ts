describe("structuredClone", () => {
  it("Clones a simple object", () => {
    const originalObject = { foo: "bar", num: 42 };
    const clonedObject = structuredClone(originalObject);

    expect(clonedObject).toStrictEqual(originalObject);
    originalObject.foo += "extra";
    expect(clonedObject).not.toStrictEqual(originalObject);
  });

  it("Clones an array", () => {
    const originalArray = [1, 2, 3, 4, 5];
    const clonedArray = structuredClone(originalArray);
    expect(clonedArray).toStrictEqual(originalArray);
  });

  it("Clones an array of objects", () => {
    let obj = { foo: "bar" };
    const originalArray = [obj, obj, obj, obj, obj];
    const clonedArray = structuredClone(originalArray);
    expect(clonedArray).toStrictEqual(originalArray);
    expect(clonedArray[0]).not.toBe(originalArray[0]);
  });

  it("Clones nested objects", () => {
    const originalObject = { foo: { bar: { baz: "qux" } } };
    const clonedObject = structuredClone(originalObject);
    expect(clonedObject).toStrictEqual(originalObject);
  });

  it("Handles circular references", () => {
    const originalObject: any = { foo: { bar: "baz", arr: [1, 2, 3] } };
    originalObject.foo.circularRef = originalObject;
    originalObject.foo.circularRef2 = originalObject;
    originalObject.foo.circularRef3 = originalObject.foo;
    originalObject.ref2 = originalObject;
    const clonedObject = structuredClone(originalObject);
    expect(clonedObject).toStrictEqual(originalObject);
  });

  it("Clones a Map", () => {
    const originalMap = new Map([
      ["key1", "value1"],
      ["key2", "value2"],
    ]);
    const clonedMap = structuredClone(originalMap);
    expect(clonedMap).toStrictEqual(originalMap);
  });

  it("Clones a Set", () => {
    const originalSet = new Set([1, 2, 3, 4, 5]);
    const clonedSet = structuredClone(originalSet);
    expect(clonedSet).toStrictEqual(originalSet);
  });

  it("Clones a Date object", () => {
    const originalDate = new Date("2022-01-31T12:00:00Z");
    const clonedDate = structuredClone(originalDate);
    expect(clonedDate.getTime()).toEqual(originalDate.getTime());
  });

  it("Clones a Buffer", () => {
    const buffer = Buffer.from("hello world");
    const clonedBuffer = structuredClone(buffer);
    expect(clonedBuffer.buffer).toEqual(buffer.buffer);
    buffer.set([1, 2, 3, 4, 5, 6, 7, 8]);
    expect(clonedBuffer).not.toStrictEqual(buffer);
  });

  it("Clones a DataView", () => {
    const buffer = new ArrayBuffer(8);
    const bytes = new Uint8Array(buffer);
    bytes.set([1, 2, 3, 4, 5, 6, 7, 8]);
    const view = new DataView(buffer, 2, 4);
    const cloned = structuredClone(view);
    expect(cloned.byteOffset).toBe(2);
    expect(cloned.byteLength).toBe(4);
    expect(cloned.buffer.byteLength).toBe(8);
    expect(Array.from(new Uint8Array(cloned.buffer))).toEqual([
      1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    // the clone is a copy, not a view of the original buffer
    bytes[2] = 99;
    expect(cloned.getUint8(0)).toBe(3);
  });

  it("Clones typed arrays", () => {
    const original = new Int16Array([1, -2, 3]);
    const cloned = structuredClone(original);
    expect(Array.from(cloned)).toEqual([1, -2, 3]);
    expect(cloned).not.toBe(original);
  });

  it("Clones typed array views with their backing buffer", () => {
    const buffer = new ArrayBuffer(16);
    new Uint8Array(buffer).set([
      1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
    ]);
    const view = new Uint8Array(buffer, 4, 4);
    const cloned = structuredClone(view);
    expect(cloned.byteOffset).toBe(4);
    expect(cloned.length).toBe(4);
    expect(cloned.buffer.byteLength).toBe(16);
    expect(Array.from(new Uint8Array(cloned.buffer))).toEqual([
      1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
    ]);
    expect(cloned.buffer).not.toBe(buffer);
  });

  it("Clones ArrayBuffers", () => {
    const buffer = new ArrayBuffer(8);
    new Uint8Array(buffer).set([1, 2, 3, 4, 5, 6, 7, 8]);
    const cloned = structuredClone(buffer);
    expect(cloned).toBeInstanceOf(ArrayBuffer);
    expect(cloned.byteLength).toBe(8);
    expect(Array.from(new Uint8Array(cloned))).toEqual([
      1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    expect(cloned).not.toBe(buffer);
  });

  it("Clones boxed primitives", () => {
    const num = structuredClone(new Number(5));
    expect(Object.prototype.toString.call(num)).toBe("[object Number]");
    expect(+num).toBe(5);

    const str = structuredClone(new String("ab"));
    expect(Object.prototype.toString.call(str)).toBe("[object String]");
    expect(str.length).toBe(2);
    expect(String(str)).toBe("ab");

    const bool = structuredClone(new Boolean(true));
    expect(Object.prototype.toString.call(bool)).toBe("[object Boolean]");
    expect(bool.valueOf()).toBe(true);
  });

  it("Clones nested collections", () => {
    const set = structuredClone(new Set([new Set([1])]));
    expect(set.values().next().value.size).toBe(1);

    const map = structuredClone(new Map([["k", new Map([[1, 2]])]]));
    expect(map.get("k").size).toBe(1);
    expect(map.get("k").get(1)).toBe(2);
  });

  it("Preserves array holes and extra properties", () => {
    const sparse = structuredClone([, , 1]);
    expect(sparse.length).toBe(3);
    expect(Object.keys(sparse)).toEqual(["2"]);
    expect(1 in sparse).toBe(false);

    const withProps: any = structuredClone(Object.assign([1], { x: 2 }));
    expect(Object.keys(withProps)).toEqual(["0", "x"]);
    expect(withProps[0]).toBe(1);
    expect(withProps.x).toBe(2);
  });

  it("Clones Blob and File", () => {
    const blob = structuredClone(new Blob(["ab"]));
    expect(blob).toBeInstanceOf(Blob);
    expect(blob.size).toBe(2);

    const file = structuredClone(
      new File(["abc"], "a.txt", { type: "text/plain", lastModified: 123 })
    );
    expect(file).toBeInstanceOf(File);
    expect(file.name).toBe("a.txt");
    expect(file.size).toBe(3);
    expect(file.type).toBe("text/plain");
    expect(file.lastModified).toBe(123);
  });

  it("Throws DataCloneError for weak collections", () => {
    expect(() => structuredClone(new WeakMap())).toThrow(/could not be cloned/);
    expect(() => structuredClone(new WeakSet())).toThrow(/could not be cloned/);
  });

  it("Handles transfer list", () => {
    const buffer = new ArrayBuffer(3);
    new Uint8Array(buffer).set([1, 2, 3]);
    const originalObject: any = { foo: { bar: "baz", buffer } };
    const clonedObject1 = structuredClone(originalObject);

    expect(clonedObject1.foo.buffer).not.toBe(originalObject.foo.buffer);

    const clonedObject2 = structuredClone(originalObject, {
      transfer: [originalObject.foo.buffer],
    });
    expect(new Uint8Array(clonedObject2.foo.buffer)).toEqual(
      new Uint8Array([1, 2, 3])
    );
  });

  it("Throws DataCloneError for un-clonable values", () => {
    expect(() => structuredClone(() => {})).toThrow(/could not be cloned/);
    expect(() => structuredClone(Symbol("x"))).toThrow(/could not be cloned/);
    expect(() => structuredClone(Promise.resolve())).toThrow(
      /could not be cloned/
    );
    // nested inside a cloneable structure
    expect(() => structuredClone({ f: () => {} })).toThrow(
      /could not be cloned/
    );
  });
});
