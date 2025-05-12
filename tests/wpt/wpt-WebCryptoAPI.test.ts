import { runTestDynamic } from "./wpt-WebCryptoAPI.harness.js";
import fs from "fs";
import path from "path";

const CWD = process.cwd();
const baseDir = `${CWD}/tests/wpt/wpt`;

const testFiles = [
  "crypto_key_cached_slots.https.any.js",
  "getRandomValues.any.js",
  "randomUUID.https.any.js",
];

describe("WebCryptoAPI", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      const filePath = path.resolve(baseDir, "WebCryptoAPI", file);
      const sourceCode = fs.readFileSync(filePath, "utf8");
      runTestDynamic(sourceCode, done);
    });
  }
});
