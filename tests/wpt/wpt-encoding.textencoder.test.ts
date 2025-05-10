import { runTestWpt } from "./runTestWpt.js";
import fs from "fs";
import path from "path";

const CWD = process.cwd();
const baseDir = `${CWD}/tests/wpt`;

const testFiles = [
  "textencoder-constructor-non-utf.any.js",
  // "textencoder-utf16-surrogates.any.js",
];

describe("textencoder", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      const filePath = path.resolve(baseDir, "encoding", file);
      const sourceCode = fs.readFileSync(filePath, "utf8");
      runTestWpt(sourceCode, done);
    });
  }
});
