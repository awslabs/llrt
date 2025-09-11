import { runTestDynamic } from "./fetch.harness.js";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SKIP_FILES = [
  "request-forbidden-headers.any.js", // ReferenceError: promise_test is not defined
  "request-private-network-headers.tentative.any.js", // ReferenceError: promise_test is not defined
  "request-referrer.any.js", // TypeError: cannot read property 'href' of undefined
  "request-upload.h2.any.js",
  "scheme-blob.sub.any.js", // TypeError: not a function
];

const __filename = fileURLToPath(import.meta.url);
const basename = path.basename(__filename);
const subDir = basename
  .replace(/\.test\.[jt]s$/, "")
  .split(".")
  .join(path.sep);

const CWD = process.cwd();
const baseDir = path.join(CWD, "wpt");
const targetDir = path.join(baseDir, subDir);

const testFiles = fs
  .readdirSync(targetDir)
  .filter((file) => file.endsWith(".any.js"));

describe(subDir, () => {
  for (const file of testFiles) {
    if (!SKIP_FILES.includes(file)) {
      it(`should pass ${file} tests`, (done) => {
        const filePath = path.join(targetDir, file);
        const sourceCode = fs.readFileSync(filePath, "utf8");
        runTestDynamic(sourceCode, baseDir, done);
      });
    }
  }
});
