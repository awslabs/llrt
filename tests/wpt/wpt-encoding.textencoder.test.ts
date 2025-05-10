import { runTest } from "./runTest.js";

const testFiles = [
  "textencoder-constructor-non-utf.any.js",
    // "textencoder-utf16-surrogates.any.js",
];

describe("textencoder", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      runTest(require(`./encoding/${file}`).default, done);
    });
  }
});
