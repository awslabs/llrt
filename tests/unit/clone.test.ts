describe("structuredClone", () => {
  it("Clones a simple object", () => {
    const originalObject = { foo: "bar", num: 42 };
    const clonedObject = structuredClone(originalObject);

    assert.deepStrictEqual(clonedObject, originalObject);
    originalObject.foo += "extra";
    assert.notDeepStrictEqual(clonedObject, originalObject);
  });

  it("Clones an array", () => {
    const originalArray = [1, 2, 3, 4, 5];
    const clonedArray = structuredClone(originalArray);
    assert.deepStrictEqual(clonedArray, originalArray);
  });

  it("Clones an array of objects", () => {
    let obj = { foo: "bar" };
    const originalArray = [obj, obj, obj, obj, obj];
    const clonedArray = structuredClone(originalArray);
    assert.deepStrictEqual(clonedArray, originalArray);
    assert.notEqual(clonedArray[0], originalArray[0]);
  });

  it("Clones nested objects", () => {
    const originalObject = { foo: { bar: { baz: "qux" } } };
    const clonedObject = structuredClone(originalObject);
    assert.deepStrictEqual(clonedObject, originalObject);
  });

  it("Handles circular references", () => {
    const originalObject: any = { foo: { bar: "baz", arr: [1, 2, 3] } };
    originalObject.foo.circularRef = originalObject;
    originalObject.foo.circularRef2 = originalObject;
    originalObject.foo.circularRef3 = originalObject.foo;
    originalObject.ref2 = originalObject;
    const clonedObject = structuredClone(originalObject);
    assert.deepStrictEqual(clonedObject, originalObject);
  });

  it("Clones a Map", () => {
    const originalMap = new Map([
      ["key1", "value1"],
      ["key2", "value2"],
    ]);
    const clonedMap = structuredClone(originalMap);
    assert.deepStrictEqual(clonedMap, originalMap);
  });

  it("Clones a Set", () => {
    const originalSet = new Set([1, 2, 3, 4, 5]);
    const clonedSet = structuredClone(originalSet);
    assert.deepStrictEqual(clonedSet, originalSet);
  });

  it("Clones a Date object", () => {
    const originalDate = new Date("2022-01-31T12:00:00Z");
    const clonedDate = structuredClone(originalDate);
    assert.strictEqual(clonedDate.getTime(), originalDate.getTime());
  });

  it("Clones a Buffer", () => {
    const buffer = Buffer.from("hello world");
    const clonedBuffer = structuredClone(buffer);
    assert.deepEqual(clonedBuffer.buffer, buffer.buffer);
    buffer.set([1, 2, 3, 4, 5, 6, 7, 8]);
    assert.notDeepStrictEqual(clonedBuffer, buffer);
  });

  it("Handles transfer list", () => {
    const originalObject: any = { foo: { bar: "baz", arr: [1, 2, 3] } };
    const clonedObject1 = structuredClone(originalObject);

    assert.notStrictEqual(clonedObject1.foo.arr, originalObject.foo.arr);

    const clonedObject2 = structuredClone(originalObject, {
      transfer: [originalObject.foo.arr],
    });
    assert.strictEqual(clonedObject2.foo.arr, originalObject.foo.arr);
  });
});
