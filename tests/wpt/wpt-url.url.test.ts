import { runTest } from "./runTest.js";

const testFiles = [
  "url-constructor.any.js",
  "url-origin.any.js",
  "url-searchparams.any.js",
  "url-setters.any.js",
  "url-setters-stripping.any.js",
  "url-statics-canparse.any.js",
  "url-statics-parse.any.js",
  "url-tojson.any.js",
];

describe("url", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      runTest(require(`./url/${file}`).default, done);
    });
  }
});
