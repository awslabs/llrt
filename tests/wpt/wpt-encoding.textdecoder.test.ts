import { runTest } from "./runTest.js";

const testFiles = [
  "textdecoder-arguments.any.js",
  "textdecoder-byte-order-marks.any.js",
  // "textdecoder-copy.any.js", // stream option not implemented
  "textdecoder-eof.any.js",
  // "textdecoder-fatal-single-byte.any.js", // Current support is utf8 and utf16le
  "textdecoder-fatal-streaming.any.js",
  "textdecoder-fatal.any.js",
  "textdecoder-ignorebom.any.js",
  "textdecoder-labels.any.js",
  // "textdecoder-streaming.any.js", // stream option not implemented
  "textdecoder-utf16-surrogates.any.js",
];

describe("textdecoder", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      runTest(require(`./encoding/${file}`).default, done);
    });
  }
});
