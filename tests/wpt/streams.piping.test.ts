import { runTestDynamic } from "./streams.harness.js";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const SKIP_FILES = [
  "general-addition.any.js", // waiting on resolution of https://github.com/whatwg/streams/issues/1243.
  "throwing-options.any.js", // requires TransformStream
  "transform-streams.any.js", // requires TransformStream
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
        runTestDynamic(sourceCode, done);
      });
    }
  }
});
