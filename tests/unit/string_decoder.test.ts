import defaultImport from "node:string_decoder";
import legacyImport from "string_decoder";
import * as legacyNamedImport from "string_decoder";

const modules = {
  "node:string_decoder": defaultImport,
  string_decoder: legacyImport,
  "* as string_decoder": legacyNamedImport,
};
for (const module in modules) {
  const { StringDecoder } = modules[module];

  describe(module, () => {
    // Taken from https://github.com/nodejs/node/blob/ea9be1787203c3186c35a1a90d7d1109aa5ea1c3/test/parallel/test-string-decoder.js
    describe("basic", () => {
      // testDecode verifies that StringDecoder will correctly decode the given input
      // buffer with the given encoding to the expected output. It will attempt all
      // possible ways to write() the input buffer, see writeSequences(). The
      // singleSequence allows for easy debugging of a specific sequence which is
      // useful in case of testDecode failures.
      function testDecode(
        encoding: BufferEncoding,
        input: Buffer,
        expected: string,
        singleSequence?: Array<Array<number>>
      ) {
        let sequences;
        if (!singleSequence) {
          sequences = writeSequences(input.length);
        } else {
          sequences = [singleSequence];
        }
        sequences.forEach((sequence) => {
          const decoder = new StringDecoder(encoding);
          let output = "";
          sequence.forEach((write) => {
            output += decoder.write(input.slice(write[0], write[1]));
          });
          output += decoder.end();
          expect(output).toEqual(expected);
        });
      }

      // writeSequences returns an array of arrays that describes all possible ways a
      // buffer of the given length could be split up and passed to sequential write
      // calls.
      //
      // e.G. writeSequences(3) will return: [
      //   [ [ 0, 3 ] ],
      //   [ [ 0, 2 ], [ 2, 3 ] ],
      //   [ [ 0, 1 ], [ 1, 3 ] ],
      //   [ [ 0, 1 ], [ 1, 2 ], [ 2, 3 ] ]
      // ]
      function writeSequences(
        length: number,
        start?: number,
        sequence?: Array<Array<number>>
      ): Array<Array<Array<number>>> {
        if (start === undefined) {
          start = 0;
          sequence = [];
        } else if (start === length) {
          return [sequence!];
        }
        let sequences: Array<Array<Array<number>>> = [];
        for (let end = length; end > start; end--) {
          const subSequence = sequence!.concat([[start, end]]);
          const subSequences = writeSequences(length, end, subSequence);
          sequences = sequences.concat(subSequences);
        }
        return sequences;
      }

      it("utf8", () => {
        testDecode("utf-8", Buffer.from("$", "utf-8"), "$");
        testDecode("utf-8", Buffer.from("¢", "utf-8"), "¢");
        testDecode("utf-8", Buffer.from("€", "utf-8"), "€");
        testDecode("utf-8", Buffer.from("𤭢", "utf-8"), "𤭢");
        // A mixed ascii and non-ascii string
        // testDecode stolen from deps/v8/testDecode/cctest/testDecode-strings.cc
        // U+02E4 -> CB A4
        // U+0064 -> 64
        // U+12E4 -> E1 8B A4
        // U+0030 -> 30
        // U+3045 -> E3 81 85
        testDecode(
          "utf-8",
          Buffer.from([
            0xcb, 0xa4, 0x64, 0xe1, 0x8b, 0xa4, 0x30, 0xe3, 0x81, 0x85,
          ]),
          "\u02e4\u0064\u12e4\u0030\u3045"
        );

        // Some invalid input, known to have caused trouble with chunking
        // in https://github.com/nodejs/node/pull/7310#issuecomment-226445923
        // 00: |00000000 ASCII
        // 41: |01000001 ASCII
        // B8: 10|111000 continuation
        // CC: 110|01100 two-byte head
        // E2: 1110|0010 three-byte head
        // F0: 11110|000 four-byte head
        // F1: 11110|001'another four-byte head
        // FB: 111110|11 "five-byte head", not UTF-8
        testDecode("utf-8", Buffer.from("C9B5A941", "hex"), "\u0275\ufffdA");
        testDecode("utf-8", Buffer.from("E2", "hex"), "\ufffd");
        testDecode("utf-8", Buffer.from("E241", "hex"), "\ufffdA");
        testDecode("utf-8", Buffer.from("CCCCB8", "hex"), "\ufffd\u0338");
        testDecode("utf-8", Buffer.from("F0B841", "hex"), "\ufffdA");
        testDecode("utf-8", Buffer.from("F1CCB8", "hex"), "\ufffd\u0338");
        testDecode("utf-8", Buffer.from("F0FB00", "hex"), "\ufffd\ufffd\0");
        testDecode("utf-8", Buffer.from("CCE2B8B8", "hex"), "\ufffd\u2e38");
        testDecode("utf-8", Buffer.from("E2B8CCB8", "hex"), "\ufffd\u0338");
        testDecode(
          "utf-8",
          Buffer.from("E2FBCC01", "hex"),
          "\ufffd\ufffd\ufffd\u0001"
        );
        testDecode("utf-8", Buffer.from("CCB8CDB9", "hex"), "\u0338\u0379");
        testDecode(
          "utf-8",
          Buffer.from("EDA0B5EDB08D", "hex"),
          "\ufffd\ufffd\ufffd\ufffd\ufffd\ufffd"
        );
      });

      it("utf8 end", () => {
        let decoder = new StringDecoder("utf8");
        expect(decoder.write(Buffer.from("E18B", "hex"))).toEqual("");
        expect(decoder.end()).toEqual("\ufffd");

        decoder = new StringDecoder("utf8");
        expect(decoder.write(Buffer.from("\ufffd"))).toEqual("\ufffd");
        expect(decoder.end()).toEqual("");

        decoder = new StringDecoder("utf8");
        expect(decoder.write(Buffer.from("\ufffd\ufffd\ufffd"))).toEqual(
          "\ufffd\ufffd\ufffd"
        );
        expect(decoder.end()).toEqual("");

        decoder = new StringDecoder("utf8");
        expect(decoder.write(Buffer.from("EFBFBDE2", "hex"))).toEqual("\ufffd");
        expect(decoder.end()).toEqual("\ufffd");

        decoder = new StringDecoder("utf8");
        expect(decoder.write(Buffer.from("F1", "hex"))).toEqual("");
        expect(decoder.write(Buffer.from("41F2", "hex"))).toEqual("\ufffdA");
        expect(decoder.end()).toEqual("\ufffd");
      });

      it("utf8 regression", () => {
        // Regression tests for https://github.com/nodejs/node/issues/22626
        // (not enough replacement chars when having seen more than one byte of an
        // incomplete multibyte characters).
        let decoder = new StringDecoder("utf8");
        expect(decoder.write(Buffer.from("f69b", "hex"))).toEqual("");
        expect(decoder.write(Buffer.from("d1", "hex"))).toEqual("\ufffd\ufffd");
        expect(decoder.end()).toEqual("\ufffd");
        expect(decoder.write(Buffer.from("f4", "hex"))).toEqual("");
        expect(decoder.write(Buffer.from("bde5", "hex"))).toEqual(
          "\ufffd\ufffd"
        );
        expect(decoder.end()).toEqual("\ufffd");
      });

      it("utf16le", () => {
        // BUG: See https://github.com/quickjs-ng/quickjs/issues/992
        // testDecode("ucs2", Buffer.from("ababc", "utf16le"), "ababc");
        testDecode("utf16le", Buffer.from("3DD84DDC", "hex"), "\ud83d\udc4d"); // thumbs up
      });

      it("utf16le surrogate", () => {
        let decoder = new StringDecoder("utf16le");
        expect(decoder.write(Buffer.from("3DD8", "hex"))).toEqual("");
        expect(decoder.write(Buffer.from("4D", "hex"))).toEqual("");
        expect(decoder.write(Buffer.from("DC", "hex"))).toEqual("\ud83d\udc4d");
        expect(decoder.end()).toEqual("");

        decoder = new StringDecoder("utf16le");
        expect(decoder.write(Buffer.from("3DD8", "hex"))).toEqual("");
        expect(decoder.end()).toEqual("\uFFFD");

        decoder = new StringDecoder("utf16le");
        expect(decoder.write(Buffer.from("3DD8", "hex"))).toEqual("");
        expect(decoder.write(Buffer.from("4D", "hex"))).toEqual("");
        expect(decoder.end()).toEqual("\uFFFD");

        decoder = new StringDecoder("utf16le");
        expect(decoder.write(Buffer.from("3DD84D", "hex"))).toEqual("");
        expect(decoder.end()).toEqual("\ufffd");
      });

      it("utf16le regression", () => {
        // Regression test for https://github.com/nodejs/node/issues/22358
        // (unaligned UTF-16 access).
        let decoder = new StringDecoder("utf16le");
        expect(decoder.write(Buffer.alloc(1))).toEqual("");
        expect(decoder.write(Buffer.alloc(20))).toEqual("\0".repeat(10));
        expect(decoder.write(Buffer.alloc(48))).toEqual("\0".repeat(24));
        expect(decoder.end()).toEqual("");
      });

      it("throws", () => {
        expect(() => new StringDecoder("test" as any)).toThrow(
          expect.objectContaining({
            name: "TypeError",
            message: "Unknown encoding: test",
          })
        );
      });
    });

    // Taken from https://github.com/nodejs/node/blob/ea9be1787203c3186c35a1a90d7d1109aa5ea1c3/test/parallel/test-string-decoder-end.js
    describe("end", () => {
      const bufs = ["☃💩", "asdf"].map((b) => Buffer.from(b));

      function testEnd(
        encoding: BufferEncoding,
        incomplete: Uint8Array,
        next: Uint8Array,
        expected: string
      ) {
        let res = "";
        const s = new StringDecoder(encoding);
        res += s.write(incomplete);
        res += s.end();
        res += s.write(next);
        res += s.end();

        expect(res).toEqual(expected);
      }

      function testEncoding(encoding: BufferEncoding) {
        bufs.forEach((buf) => {
          testBuf(encoding, buf);
        });
      }

      function testBuf(encoding: BufferEncoding, buf: Buffer) {
        // Write one byte at a time.
        let s = new StringDecoder(encoding);
        let res1 = "";
        for (let i = 0; i < buf.length; i++) {
          res1 += s.write(buf.slice(i, i + 1));
        }
        res1 += s.end();

        // Write the whole buffer at once.
        let res2 = "";
        s = new StringDecoder(encoding);
        res2 += s.write(buf);
        res2 += s.end();

        // .toString() on the buffer
        const res3 = buf.toString(encoding);

        // One byte at a time should match toString
        expect(res1).toEqual(res3);
        // All bytes at once should match toString
        expect(res2).toEqual(res3);
      }

      it("encodings", () => {
        const encodings: BufferEncoding[] = [
          "base64",
          "hex",
          "utf8",
          "utf16le",
        ];
        encodings.forEach(testEncoding);
      });

      it("utf8", () => {
        testEnd("utf8", Buffer.of(0xe2), Buffer.of(0x61), "\uFFFDa");
        testEnd("utf8", Buffer.of(0xe2), Buffer.of(0x82), "\uFFFD\uFFFD");
        testEnd("utf8", Buffer.of(0xe2), Buffer.of(0xe2), "\uFFFD\uFFFD");
        testEnd("utf8", Buffer.of(0xe2, 0x82), Buffer.of(0x61), "\uFFFDa");
        testEnd("utf8", Buffer.of(0xe2, 0x82), Buffer.of(0xac), "\uFFFD\uFFFD");
        testEnd("utf8", Buffer.of(0xe2, 0x82), Buffer.of(0xe2), "\uFFFD\uFFFD");
        testEnd("utf8", Buffer.of(0xe2, 0x82, 0xac), Buffer.of(0x61), "€a");
      });

      it("utf16le", () => {
        testEnd("utf16le", Buffer.of(0x3d), Buffer.of(0x61, 0x00), "a");
        testEnd(
          "utf16le",
          Buffer.of(0x3d),
          Buffer.of(0xd8, 0x4d, 0xdc),
          "\u4DD8"
        );
        testEnd("utf16le", Buffer.of(0x3d, 0xd8), Buffer.of(), "\uFFFD");
        testEnd(
          "utf16le",
          Buffer.of(0x3d, 0xd8),
          Buffer.of(0x61, 0x00),
          "\uFFFDa"
        );
        testEnd(
          "utf16le",
          Buffer.of(0x3d, 0xd8),
          Buffer.of(0x4d, 0xdc),
          "\uFFFD\uFFFD"
        );
        testEnd("utf16le", Buffer.of(0x3d, 0xd8, 0x4d), Buffer.of(), "\uFFFD");
        testEnd(
          "utf16le",
          Buffer.of(0x3d, 0xd8, 0x4d),
          Buffer.of(0x61, 0x00),
          "\uFFFDa"
        );
        testEnd(
          "utf16le",
          Buffer.of(0x3d, 0xd8, 0x4d),
          Buffer.of(0xdc),
          "\uFFFD"
        );
        testEnd(
          "utf16le",
          Buffer.of(0x3d, 0xd8, 0x4d, 0xdc),
          Buffer.of(0x61, 0x00),
          "👍a"
        );
      });

      it("base64", () => {
        testEnd("base64", Buffer.of(0x61), Buffer.of(), "YQ==");
        testEnd("base64", Buffer.of(0x61), Buffer.of(0x61), "YQ==YQ==");
        testEnd("base64", Buffer.of(0x61, 0x61), Buffer.of(), "YWE=");
        testEnd("base64", Buffer.of(0x61, 0x61), Buffer.of(0x61), "YWE=YQ==");
        testEnd("base64", Buffer.of(0x61, 0x61, 0x61), Buffer.of(), "YWFh");
        testEnd(
          "base64",
          Buffer.of(0x61, 0x61, 0x61),
          Buffer.of(0x61),
          "YWFhYQ=="
        );
      });
    });
  });
}
