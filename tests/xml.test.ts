import { XMLParser } from "xml";

describe("XMLParser options and handling", () => {
  it("should parse xml", () => {
    const xmlString =
      '<root><person occupation="programmer">John</person></root>';
    const expectedResult = {
      root: { person: "John" },
    };
    const parser = new XMLParser();
    const result = parser.parse(xmlString);
    assert.deepStrictEqual(result, expectedResult);
  });

  it("should apply attributeValueProcessor", () => {
    const xmlString =
      '<root><person occupation="programmer">John</person></root>';
    const expectedResult = {
      root: { person: { _attr_occupation: "PROGRAMMER", name: "John" } },
    };
    const parser = new XMLParser({
      ignoreAttributes: false,
      attributeNamePrefix: "_attr_",
      textNodeName: "name",
      attributeValueProcessor: (_, val) => val.toUpperCase(),
    });
    const result = parser.parse(xmlString);
    assert.deepStrictEqual(result, expectedResult);
  });

  it("should apply tagValueProcessor", () => {
    const xmlString = "<root><name><![CDATA[John]]></name></root>";
    const expectedResult = {
      root: {
        name: "JOHN",
      },
    };
    const parser = new XMLParser({
      tagValueProcessor: (_, val) => val.toUpperCase(),
    });
    const result = parser.parse(xmlString);
    assert.deepStrictEqual(result, expectedResult);
  });

  it('should handle attributeNamePrefix with default "@"', () => {
    const xmlString = '<root><person first_name="John" /></root>';
    const expectedResult = {
      root: {
        person: {
          "@_first_name": "John",
        },
      },
    };
    const parser = new XMLParser({
      ignoreAttributes: false,
    });
    const result = parser.parse(xmlString);
    assert.deepStrictEqual(result, expectedResult);
  });

  it("should handle custom attributeNamePrefix", () => {
    const xmlString = '<root><person first_name="John" /></root>';
    const expectedResult = {
      root: {
        person: {
          "#first_name": "John",
        },
      },
    };
    const parser = new XMLParser({
      attributeNamePrefix: "#",
      ignoreAttributes: false,
    });
    const result = parser.parse(xmlString);
    assert.deepStrictEqual(result, expectedResult);
  });

  it("should handle siblings with the same tag name as an array", () => {
    const xmlString =
      "<root><person>John</person><person>Alice</person></root>";
    const expectedResult = {
      root: {
        person: ["John", "Alice"],
      },
    };
    const parser = new XMLParser();
    const result = parser.parse(xmlString);
    assert.deepStrictEqual(result, expectedResult);
  });

  it("should handle attributes and text content for sibling arrays", () => {
    const xmlString =
      '<root><person name="John">Developer</person><person name="Alice">Designer</person></root>';
    const expectedResult = {
      root: {
        person: [
          { "@_name": "John", "#text": "Developer" },
          { "@_name": "Alice", "#text": "Designer" },
        ],
      },
    };
    const parser = new XMLParser({
      ignoreAttributes: false,
    });
    const result = parser.parse(xmlString);
    assert.deepStrictEqual(result, expectedResult);
  });

  it("should handle attributes and text content for sibling arrays for empty tags", () => {
    const xmlString =
      '<root><person name="John"/><person name="Alice"/></root>';
    const expectedResult = {
      root: { person: [{ "@_name": "John" }, { "@_name": "Alice" }] },
    };
    const parser = new XMLParser({
      ignoreAttributes: false,
    });
    const result = parser.parse(xmlString);
    assert.deepStrictEqual(result, expectedResult);
  });

  it("should handle attributes and text content for sibling arrays", () => {
    const xmlString =
      '<root><person>John</person><person role="Designer">Alice</person></root>';
    const expectedResult = {
      root: { person: ["John", { "@_role": "Designer", "#text": "Alice" }] },
    };
    const parser = new XMLParser({
      ignoreAttributes: false,
    });
    const result = parser.parse(xmlString);
    assert.deepStrictEqual(result, expectedResult);
  });
  it("should handle attributes and text content for different objects and siblings", () => {
    const xmlString =
      "<root><person>John</person><person>Alice</person><group>Developers</group></root>";
    const expectedResult = {
      root: {
        person: ["John", "Alice"],
        group: "Developers",
      },
    };
    const parser = new XMLParser({
      ignoreAttributes: false,
    });
    const result = parser.parse(xmlString);
    assert.deepStrictEqual(result, expectedResult);
  });
});
