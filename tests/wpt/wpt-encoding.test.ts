import { runTestDynamic } from "./wptTestHarness.js";
import fs from "fs";
import path from "path";

const CWD = process.cwd();
const baseDir = `${CWD}/tests/wpt`;

const testFiles = [
  "api-basics.any.js",
  "api-invalid-label.any.js",
  "api-replacement-encodings.any.js",
  "api-surrogates-utf8.any.js",
  "encodeInto.any.js",
  "replacement-encodings.any.js",
  "unsupported-encodings.any.js",
];

describe("encoding", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      const filePath = path.resolve(baseDir, "encoding", file);
      const sourceCode = fs.readFileSync(filePath, "utf8");
      runTestDynamic(sourceCode, done);
    });
  }
});
