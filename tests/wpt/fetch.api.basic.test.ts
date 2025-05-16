import { runTestDynamic } from "./fetch.harness.js";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const SKIP_FILES = [
  "integrity.sub.any.js", // TypeError: cannot read property 'replace' of undefined
  "keepalive.any.js", // ReferenceError: get_host_info is not defined
  "mode-same-origin.any.js", // ReferenceError: get_host_info is not defined
  "referrer.any.js", // ReferenceError: get_host_info is not defined
  "request-forbidden-headers.any.js", // ReferenceError: promise_test is not defined
  "request-private-network-headers.tentative.any.js", // ReferenceError: promise_test is not defined
  "request-referrer.any.js", // TypeError: cannot read property 'href' of undefined
  "scheme-blob.sub.any.js", // TypeError: not a function
];

const __filename = fileURLToPath(import.meta.url);
const basename = path.basename(__filename);
const subDir = basename
  .replace(/\.test\.[jt]s$/, "")
  .split(".")
  .join(path.sep);

const CWD = process.cwd();
const baseDir = path.join(CWD, "tests", "wpt", "wpt");
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
        runTestDynamic(sourceCode, done);
      });
    }
  }
});
