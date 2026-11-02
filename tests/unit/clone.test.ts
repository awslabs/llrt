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
