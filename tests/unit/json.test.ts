describe("JSON Parsing", () => {
  it("should parse valid JSON", () => {
    const parsedData = JSON.parse('{"key": "value"}');
    expect(parsedData).toStrictEqual({ key: "value" });
  });

  it("should handle invalid JSON", () => {
    const invalidJsonString = '{key: "value"}';
    expect(() => {
      JSON.parse(invalidJsonString);
    }).toThrow();

    const emptyJsonString = "";
    expect(() => {
      JSON.parse(emptyJsonString);
    }).toThrow();
  });

  it("should parse JSON with nested structures", () => {
    const parsedData = JSON.parse(
      '{"name": "John", "age": 25, "address": {"city": "New York", "zip": "10001"}}'
    );
    expect(parsedData).toStrictEqual({
      name: "John",
      age: 25,
      address: { city: "New York", zip: "10001" },
    });
  });

  it("should parse JSON with arrays", () => {
    const parsedData = JSON.parse('[1, 2, 3, {"key": "value"}]');
    expect(parsedData).toStrictEqual([1, 2, 3, { key: "value" }]);
  });

  it("should parse JSON with boolean values", () => {
    const parsedData = JSON.parse('{"isTrue": true, "isFalse": false}');
    expect(parsedData).toStrictEqual({ isTrue: true, isFalse: false });
  });

  it("should parse JSON with null values", () => {
    const parsedData = JSON.parse('{"nullableValue": null}');
    expect(parsedData).toStrictEqual({ nullableValue: null });
  });

  it("should parse JSON with large int value", () => {
    const parsedData = JSON.parse('{"bigInt": 888888888888888888}');
    expect(parsedData).toStrictEqual({ bigInt: 888888888888888888 });
  });

  it("should parse JSON with special characters", () => {
    const specialChars = "!@#$%^&*()_+-={}[]|;:,.<>?/";
    const parsedData = JSON.parse(`{"specialChars": "${specialChars}"}`);
    expect(parsedData).toStrictEqual({ specialChars });
  });
});

describe("JSON Stringified", () => {
  it("should stringify JSON", () => {
    const data = { key: "value", age: 25 };
    const jsonString = JSON.stringify(data);
    const parsedData = JSON.parse(jsonString);
    expect(parsedData).toStrictEqual(data);
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
    expect(parsedData).toStrictEqual({ customKey: "VALUE", customAge: 50 });
  });

  it("should print floats without fractions as integers", () => {
    const jsonString = JSON.stringify({ value: 1.0 });
    expect(jsonString).toEqual('{"value":1}');
  });

  it("should print very large numbers as floats with scientific notation", () => {
    const jsonString = JSON.stringify({
      value: 999999999999999999999999999999,
    });
    expect(jsonString).toEqual('{"value":1e30}');
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

    expect(() => {
      JSON.stringify(recursiveData);
    }).toThrow();
  });

  it("Should stringify an object with default spacing", () => {
    const data = {
      key: "value",
      bool: true,
      num: 42,
      arr: [1, 2, 3],
      nested: {
        level1: {
          level2: {
            level3: "nestedValue",
          },
        },
      },
    };
    const jsonString = JSON.stringify(data, null, 4);
    const expectedJsonString = `{
    "key": "value",
    "bool": true,
    "num": 42,
    "arr": [
        1,
        2,
        3
    ],
    "nested": {
        "level1": {
            "level2": {
                "level3": "nestedValue"
            }
        }
    }
}`;
    expect(jsonString).toEqual(expectedJsonString);
  });

  // Test JSON stringifying with custom spacing as a string
  it("Should stringify an object with default custom spacing", () => {
    const data = {
      key: "value",
      bool: false,
      num: 3.14,
      arr: ["apple", "banana", "cherry"],
      nested: {
        level1: {
          level2: {
            level3: "nestedValue",
          },
        },
      },
    };
    const jsonString = JSON.stringify(data, null, "   ");
    const expectedJsonString = `{
   "key": "value",
   "bool": false,
   "num": 3.14,
   "arr": [
      "apple",
      "banana",
      "cherry"
   ],
   "nested": {
      "level1": {
         "level2": {
            "level3": "nestedValue"
         }
      }
   }
}`;
    expect(jsonString).toEqual(expectedJsonString);
  });

  // Test JSON stringifying with replacer as a function
  it("Should stringify an object with a replacer function", () => {
    const data = { key: "value", secret: "hidden" };
    const replacerFunction = (key: string, value: any) =>
      key === "secret" ? undefined : value;
    const jsonString = JSON.stringify(data, replacerFunction, 2);
    expect(jsonString).toEqual('{\n  "key": "value"\n}');
  });

  // Test more complex JSON structure
  it("Should stringify a complex object with custom spacing and replacer", () => {
    const date = new Date();
    class Foo {
      prop = 1;
    }
    const complexData = {
      key: "value",
      date,
      nested: {
        array: [1, 2, 3],
        obj: { a: "apple", b: "banana" },
        foo: new Foo(),
        arrowFn: () => {},
        fn: function () {},
        namedFn: function namedFn() {},
      },
    };

    const replacerFunction = (key: string, value: any) =>
      typeof value === "string" ? value.toUpperCase() : value;

    const jsonString = JSON.stringify(complexData, replacerFunction, 4);

    console.log(jsonString);

    const expectedJsonString = `{
    "key": "VALUE",
    "date": "${date.toJSON()}",
    "nested": {
        "array": [
            1,
            2,
            3
        ],
        "obj": {
            "a": "APPLE",
            "b": "BANANA"
        },
        "foo": {
            "prop": 1
        }
    }
}`;

    expect(jsonString).toEqual(expectedJsonString);
  });

  it("should stringify objects with undefined values", () => {
    const valueStart = {
      a: undefined,
      b: "123",
      c: "123",
    };
    expect(JSON.stringify(valueStart)).toEqual(`{"b":"123","c":"123"}`);

    const jsonStartIndented = JSON.stringify(valueStart, null, "   ");
    const expectedJsonStartString = `{
   "b": "123",
   "c": "123"
}`;
    expect(jsonStartIndented).toEqual(expectedJsonStartString);

    const valueMiddle = {
      a: "123",
      b: undefined,
      c: "123",
    };
    expect(JSON.stringify(valueMiddle)).toEqual(`{"a":"123","c":"123"}`);

    const jsonMiddleIndented = JSON.stringify(valueMiddle, null, "   ");
    const expectedJsonMiddleString = `{
   "a": "123",
   "c": "123"
}`;
    expect(jsonMiddleIndented).toEqual(expectedJsonMiddleString);

    const valueEnd = {
      a: "123",
      b: "123",
      c: undefined,
    };
    expect(JSON.stringify(valueEnd)).toEqual(`{"a":"123","b":"123"}`);

    const jsonEndIndented = JSON.stringify(valueEnd, null, "   ");
    const expectedJsonEndString = `{
   "a": "123",
   "b": "123"
}`;
    expect(jsonEndIndented).toEqual(expectedJsonEndString);

    const valueMixed = JSON.stringify({
      a: "123",
      b: undefined,
      c: undefined,
      d: "123",
      e: undefined,
      f: "123",
      g: "123",
      h: undefined,
      i: undefined,
    });

    expect(valueMixed).toEqual(`{"a":"123","d":"123","f":"123","g":"123"}`);
  });

  it("should stringify arrays with undefined values", () => {
    const valueStart = [undefined, "123", "123"];
    expect(JSON.stringify(valueStart)).toEqual(`[null,"123","123"]`);

    const valueMiddle = ["123", undefined, "123"];
    expect(JSON.stringify(valueMiddle)).toEqual(`["123",null,"123"]`);

    const valueEnd = ["123", "123", undefined];
    expect(JSON.stringify(valueEnd)).toEqual(`["123","123",null]`);
  });

  it("should stringify and remove objects that are not valid json", () => {
    const dateString = new Date().toJSON();
    const data = {
      a: "123",
      b: undefined,
      c: () => {
        return "123";
      },
      d: RegExp("apa"),
      e: new Date(),
    };
    expect(JSON.stringify(data)).toEqual(
      `{"a":"123","d":{},"e":"${dateString}"}`
    );
  });
});
