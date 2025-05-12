import { runTestDynamic } from "./wpt-streams.harness.js";
import fs from "fs";
import path from "path";

const CWD = process.cwd();
const baseDir = `${CWD}/tests/wpt/wpt`;

const testFiles = [
  "bad-buffers-and-views.any.js",
  "construct-byob-request.any.js",
  "enqueue-with-detached-buffer.any.js",
  "general.any.js",
  // SKIP: WebAssembly support is pending
  // "non-transferable-buffers.any.js",
  "patched-global.any.js",
  "read-min.any.js",
  "respond-after-enqueue.any.js",
  "tee.any.js",
  "templated.any.js",
];

describe("streams.readable-byte-streams", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      const filePath = path.resolve(
        baseDir,
        "streams",
        "readable-byte-streams",
        file
      );
      const sourceCode = fs.readFileSync(filePath, "utf8");
      runTestDynamic(sourceCode, done);
    });
  }
});
