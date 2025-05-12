import { runTestDynamic } from "./wpt-FileAPI.harness.js";
import fs from "fs";
import path from "path";

const CWD = process.cwd();
const baseDir = `${CWD}/tests/wpt/wpt`;

const testFiles = [
  "File-constructor.any.js",
  "send-file-formdata-controls.any.js",
  "send-file-formdata-punctuation.any.js",
  "send-file-formdata-utf-8.any.js",
  "send-file-formdata.any.js",
];

describe("FileAPI.file", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      const filePath = path.resolve(baseDir, "FileAPI", "file", file);
      const sourceCode = fs.readFileSync(filePath, "utf8");
      runTestDynamic(sourceCode, done);
    });
  }
});
