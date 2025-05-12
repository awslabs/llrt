import { runTestDynamic } from "./wpt-FileAPI.harness.js";
import fs from "fs";
import path from "path";

const CWD = process.cwd();
const baseDir = `${CWD}/tests/wpt/wpt`;

const testFiles = [
  "Blob-array-buffer.any.js",
  "Blob-bytes.any.js",
  "Blob-constructor.any.js",
  "Blob-slice-overflow.any.js",
  "Blob-slice.any.js",
  "Blob-stream.any.js",
  "Blob-text.any.js",
];

describe("FileAPI.blob", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      const filePath = path.resolve(baseDir, "FileAPI", "blob", file);
      const sourceCode = fs.readFileSync(filePath, "utf8");
      runTestDynamic(sourceCode, done);
    });
  }
});
