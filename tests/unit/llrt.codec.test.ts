import {
  decodeFromBase64,
  encodeToBase64,
  decodeFromHex,
  encodeToHex,
} from "llrt:codec";

describe("llrt:codec", () => {
  it("should encode/decode base64", () => {
    const hello = new Uint8Array([104, 101, 108, 108, 111]);
    const encoded = encodeToBase64(hello);
    expect(encoded).toEqual("aGVsbG8=");

    const decoded = decodeFromBase64(encoded);
    expect(decoded).toEqual(hello);
  });

  it("should encode/decode hex", () => {
    const hello = new Uint8Array([104, 101, 108, 108, 111]);
    const encoded = encodeToHex(hello);
    expect(encoded).toEqual("68656c6c6f");

    const decoded = decodeFromHex(encoded);
    expect(decoded).toEqual(hello);
  });
});
