describe("JSON Parsing", () => {
  it("should parse valid JSON", () => {
    const parsedData = JSON.parse('{"key": "value"}');
    assert.deepStrictEqual(parsedData, { key: "value" });
  });

  it("should handle invalid JSON", () => {
    const invalidJsonString = '{key: "value"}';
    assert.throws(() => {
      JSON.parse(invalidJsonString);
    });
  });

  it("should return null for empty string", () => {
    const parsedData = JSON.parse("");
    assert.strictEqual(parsedData, null);
  });

  it("should parse JSON with nested structures", () => {
    const parsedData = JSON.parse(
      '{"name": "John", "age": 25, "address": {"city": "New York", "zip": "10001"}}'
    );
    assert.deepStrictEqual(parsedData, {
      name: "John",
      age: 25,
      address: { city: "New York", zip: "10001" },
    });
  });

  it("should parse JSON with arrays", () => {
    const parsedData = JSON.parse('[1, 2, 3, {"key": "value"}]');
    assert.deepStrictEqual(parsedData, [1, 2, 3, { key: "value" }]);
  });

  it("should parse JSON with boolean values", () => {
    const parsedData = JSON.parse('{"isTrue": true, "isFalse": false}');
    assert.deepStrictEqual(parsedData, { isTrue: true, isFalse: false });
  });

  it("should parse JSON with null values", () => {
    const parsedData = JSON.parse('{"nullableValue": null}');
    assert.deepStrictEqual(parsedData, { nullableValue: null });
  });

  it("should parse JSON with special characters", () => {
    const specialChars = "!@#$%^&*()_+-={}[]|;:,.<>?/";
    const parsedData = JSON.parse(`{"specialChars": "${specialChars}"}`);
    assert.deepStrictEqual(parsedData, {
      specialChars,
    });
  });

  it("should stringify JSON", () => {
    const data = { key: "value", age: 25 };
    const jsonString = JSON.stringify(data);
    const parsedData = JSON.parse(jsonString);
    assert.deepStrictEqual(parsedData, data);
  });

  it("should handle toJSON method on regular objects", () => {
    const objWithToJSON = {
      key: "value",
      age: 25,
      toJSON() {
        return { customKey: this.key.toUpperCase(), customAge: this.age * 2 };
      },
    };

    const parsedData = JSON.parse(JSON.stringify(objWithToJSON));
    assert.deepStrictEqual(parsedData, { customKey: "VALUE", customAge: 50 });
  });

  it("should stringify and parse recursive JSON with self-referencing structures", () => {
    const recursiveData: any = {
      key: "value",
      nested: {
        age: 25,
        inner: null, // self-reference
      },
    };

    recursiveData.nested.inner = recursiveData; // create self-reference

    assert.throws(() => {
      JSON.stringify(recursiveData);
    });
  });
});
