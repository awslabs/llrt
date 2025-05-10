import { runTest } from "./runTest.js";

const testFiles = [
  "api-basics.any.js",
  "api-invalid-label.any.js",
  "api-replacement-encodings.any.js",
  "api-surrogates-utf8.any.js",
  "encodeInto.any.js",
  // "iso-2022-jp-decoder.any.js", // Current support is utf8 and utf16le
  // "replacement-encodings.any.js", // Requires XMLHTTPRequest which is not defined
  // "unsupported-encodings.any.js", // Requires XMLHTTPRequest which is not defined
];

describe("encoding", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      runTest(require(`./encoding/${file}`).default, done);
    });
  }
});
