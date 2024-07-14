import hex from "llrt:hex";

describe("llrt:hex", () => {
  it("should encode/decode text", () => {
    let hello = "hello";
    const encoded = new TextEncoder().encode(hello);
    const decoded = new TextDecoder().decode(encoded);

    expect(decoded).toEqual(hello);
  });

  it("should encode/decode hex", () => {
    const byteArray = new TextEncoder().encode("hello");
    const encoded = hex.encode(byteArray);

    expect(encoded).toEqual("68656c6c6f");
  });
});

describe("atoa & btoa", () => {
  it("btoa/atob", () => {
    const text = "Hello, world!";
    const encodedData = btoa(text);
    expect(encodedData).toEqual("SGVsbG8sIHdvcmxkIQ==");
    const decodedData = atob(encodedData);
    expect(decodedData).toEqual(text);
  });
});

describe("TextDecoder", () => {
  it("should be able to decode UTF-16LE labels", () => {
    const ary_u8 = new Uint8Array([
      0x00, 0x48, 0x20, 0xac, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x20,
      0x00, 0x77, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x6c, 0x00, 0x64, 0x00, 0x21,
    ]);
    const decoded = new TextDecoder("UTF-16LE");
    expect(decoded.encoding).toEqual("utf-16le");
    expect(decoded.decode(ary_u8)).toEqual("H€llo world!");
  });

  it("should be generated unsupported error", () => {
    try {
      const decoded = new TextDecoder("nonexistent_label");
    } catch (ex) {
      expect(ex.message).toEqual("Unsupported encoding: Nonexistentlabel");
    }
  });
});
