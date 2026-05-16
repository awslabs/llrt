import defaultImport from "node:util";
import legacyImport from "util";

import { EventEmitter } from "node:events";

it("node:util should be the same as util", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const { inherits } = defaultImport;

describe("inherits", () => {
  it("should be inheritable parent classes", () => {
    function MyStream() {
      EventEmitter.call(this);
    }

    inherits(MyStream, EventEmitter);

    const stream = new MyStream();

    expect(stream instanceof EventEmitter).toBeTruthy();
    expect(MyStream.super_).toEqual(EventEmitter);
  });
});

describe("TextDecoder", () => {
  it("should be able to decode UTF-16LE labels", () => {
    const ary_u8 = new Uint8Array([
      0x48, 0x00, 0xac, 0x20, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x20, 0x00,
      0x77, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x6c, 0x00, 0x64, 0x00, 0x21, 0x00,
    ]);
    const decoded = new TextDecoder("UTF-16LE");
    expect(decoded.encoding).toEqual("utf-16le");
    expect(decoded.decode(ary_u8)).toEqual("H€llo world!");
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
      expect(ex.message).toEqual(
        "invalid utf-8 sequence of 1 bytes from index 0"
      );
    }
  });

  it("should be generated unsupported error", () => {
    try {
      const decoded = new TextDecoder("nonexistent_label");
    } catch (ex) {
      expect(ex.message).toEqual(
        'The "nonexistent_label" encoding is not supported'
      );
    }
  });
});

describe("TextEncoder", () => {
  it("should be able to encodeInto of surrogate pair character(Short Array)", () => {
    const hono = "🔥";
    const encoded = new TextEncoder();

    const u8Array3 = new Uint8Array(3);
    const result3 = encoded.encodeInto(hono, u8Array3);
    expect(result3.read).toEqual(0);
    expect(result3.written).toEqual(0);
    expect(u8Array3).toEqual(new Uint8Array([0, 0, 0]));
  });

  it("should be able to encodeInto of surrogate pair character(Equal Length Array)", () => {
    const hono = "🔥";
    const encoded = new TextEncoder();

    const u8Array4 = new Uint8Array(4);
    const result4 = encoded.encodeInto(hono, u8Array4);
    expect(result4.read).toEqual(2);
    expect(result4.written).toEqual(4);
    expect(u8Array4).toEqual(new Uint8Array([240, 159, 148, 165]));
  });

  it("should be able to encodeInto of surrogate pair character(Long Array)", () => {
    const hono = "🔥";
    const encoded = new TextEncoder();

    const u8Array5 = new Uint8Array(5);
    const result5 = encoded.encodeInto(hono, u8Array5);
    expect(result5.read).toEqual(2);
    expect(result5.written).toEqual(4);
    expect(u8Array5).toEqual(new Uint8Array([240, 159, 148, 165, 0]));
  });

  it("should be able to encodeInto and decode", () => {
    const hono = "hono - [炎] means flame🔥 in Japanese";
    const encoded = new TextEncoder();

    const u8Array40 = new Uint8Array(40);
    const resultHono = encoded.encodeInto(hono, u8Array40);
    expect(resultHono.read).toEqual(36);
    expect(resultHono.written).toEqual(40);
    expect(u8Array40).toEqual(
      new Uint8Array([
        104, 111, 110, 111, 32, 45, 32, 91, 231, 130, 142, 93, 32, 109, 101, 97,
        110, 115, 32, 102, 108, 97, 109, 101, 240, 159, 148, 165, 32, 105, 110,
        32, 74, 97, 112, 97, 110, 101, 115, 101,
      ])
    );
    expect(new TextDecoder().decode(u8Array40)).toEqual(hono);
  });
});
