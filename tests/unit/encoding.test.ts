import hex from "hex";

describe("hex", () => {
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
  it("Should be able to decode even non UTF-8 labels ", () => {
    const hono = "炎"; // hono - [炎] means flame🔥 in Japanese
    const honoSjis = new Uint8Array([0x89, 0x8a]);
    const decoded = new TextDecoder("sjis");
    expect(decoded.encoding).toEqual("shift-jis");
    expect(decoded.fatal).toBeFalsy();
    expect(decoded.ignoreBOM).toBeFalsy();
    expect(decoded.decode(honoSjis)).toEqual(hono);
  });

  it("should not be removed BOM", () => {
    const smile = "😄";
    const bomPlusSmile = new Uint8Array([0xef, 0xbb, 0xbf, 240, 159, 152, 132]);
    expect(new TextDecoder("utf8").decode(bomPlusSmile)).toEqual(smile);

    const decoded = new TextDecoder("utf8", { ignoreBOM: true });
    expect(decoded.encoding).toEqual("utf-8");
    expect(decoded.ignoreBOM).toBeTruthy();
    const encoded = new TextEncoder().encode(decoded.decode(bomPlusSmile));
    expect(encoded).toEqual(bomPlusSmile);
  });

  it("should be generated fatal error", () => {
    const illegalString = new Uint8Array([0xff, 0xfe, 0xfd]);
    try {
      const decoded = new TextDecoder("utf-8", { fatal: true });
      expect(decoded.fatal).toBeTruthy();
      const a = decoded.decode(illegalString);
    } catch (ex) {
      expect(ex.message).toEqual("Fatal error");
    }
  });

  it("should be generated unsupported error", () => {
    try {
      const decoded = new TextDecoder("nonexistent_label");
    } catch (ex) {
      expect(ex.message).toEqual("Unsupported encoding label");
    }
  });
});
