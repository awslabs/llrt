import { runTestDynamic } from "./fetch.harness.js";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const SKIP_FILES = [
  "cache.https.any.js", // ReferenceError: caches is not defined
  "request.any.js", // ReferenceError: FormData is not defined
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
