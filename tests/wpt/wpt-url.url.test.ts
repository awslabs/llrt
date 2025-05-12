import { runTestDynamic } from "./wptTestHarness.js";
import fs from "fs";
import path from "path";

const CWD = process.cwd();
const baseDir = `${CWD}/tests/wpt/wpt`;

const testFiles = [
  "url-constructor.any.js",
  "url-origin.any.js",
  "url-searchparams.any.js",
  "url-setters.any.js",
  "url-setters-stripping.any.js",
  "url-statics-canparse.any.js",
  "url-statics-parse.any.js",
  "url-tojson.any.js",
  "urlencoded-parser.any.js",
];

describe("url", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      const filePath = path.resolve(baseDir, "url", file);
      const sourceCode = fs.readFileSync(filePath, "utf8");
      runTestDynamic(sourceCode, done);
    });
  }
});
