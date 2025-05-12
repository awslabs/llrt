import { runTestDynamic } from "./wpt-url.harness.js";
import fs from "fs";
import path from "path";

const CWD = process.cwd();
const baseDir = `${CWD}/tests/wpt/wpt`;

const testFiles = [
  "urlsearchparams-append.any.js",
  "urlsearchparams-constructor.any.js",
  "urlsearchparams-delete.any.js",
  "urlsearchparams-foreach.any.js",
  "urlsearchparams-get.any.js",
  "urlsearchparams-getall.any.js",
  "urlsearchparams-has.any.js",
  "urlsearchparams-set.any.js",
  "urlsearchparams-size.any.js",
  "urlsearchparams-sort.any.js",
  "urlsearchparams-stringifier.any.js",
];

describe("urlsearchparams", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      const filePath = path.resolve(baseDir, "url", file);
      const sourceCode = fs.readFileSync(filePath, "utf8");
      runTestDynamic(sourceCode, done);
    });
  }
});
