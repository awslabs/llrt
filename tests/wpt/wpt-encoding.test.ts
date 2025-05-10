import { runTestWpt } from "./runTestWpt.js";
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
  // "iso-2022-jp-decoder.any.js", // Current support is utf8 and utf16le
  // "replacement-encodings.any.js", // Requires XMLHTTPRequest which is not defined
  // "unsupported-encodings.any.js", // Requires XMLHTTPRequest which is not defined
];

describe("encoding", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      const filePath = path.resolve(baseDir, "encoding", file);
      const sourceCode = fs.readFileSync(filePath, "utf8");
      runTestWpt(sourceCode, done);
    });
  }
});
