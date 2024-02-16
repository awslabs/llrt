import { XMLParser, XmlText, XmlNode } from "xml";

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

  it("should handle empty tag attributes", () => {
    const xmlString = '<root><person name="John"/></root>';
    const expectedResult = {
      root: {
        person: {
          "@_name": "John",
        },
      },
    };
    const parser = new XMLParser({ ignoreAttributes: false });
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

  it.skip("should handle empty child tags", () => {
    const xmlString = "<data><prefix></prefix><name></name><empty/></data>";
    const expectedResult = {
      data: { prefix: "", name: "" },
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

describe("XML Builder", () => {
  it("Can create XmlText with escaped values", () => {
    let xml = new XmlText("<john>doe</john>").toString();

    assert.equal(xml, "&lt;john&gt;doe&lt;/john&gt;");
  });

  it("Can build XML with empty tag", () => {
    let xml = new XmlNode("data").toString();

    assert.equal(xml, "<data/>");
  });

  it("Can build XML with child", () => {
    let xml = new XmlNode("data", ["example"]).toString();

    assert.equal(xml, "<data>example</data>");
  });

  it("Can build XML with nested child", () => {
    let xml = new XmlNode("root", ["example"]);

    const node = XmlNode.of("expression", "foo").withName("expression");
    const node2 = XmlNode.of("expression2", "bar").withName("expression");
    xml.addChildNode(node);
    node.addChildNode(node2);

    assert.equal(
      xml.toString(),
      "<root>example<expression>foo<expression>bar</expression></expression></root>"
    );
  });

  it("Can build XML with deeply nested child", () => {
    let xml = new XmlNode("root");
    const node = XmlNode.of("level1");
    const node2 = XmlNode.of("level2");
    const node3 = XmlNode.of("level3", "foobar");
    xml.addChildNode(node);
    node.addChildNode(node2);
    node2.addChildNode(node3);

    assert.equal(
      xml.toString(),
      "<root><level1><level2><level3>foobar</level3></level2></level1></root>"
    );
  });

  it("Can build XML with attributes", () => {
    let xml = XmlNode.of("root")
      .addAttribute("example", "data")
      .addAttribute("example2", "data2")
      .addAttribute("example3", "data3")
      .removeAttribute("example3");

    assert.equal(xml.toString(), '<root example="data" example2="data2"/>');
  });
});
