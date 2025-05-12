import { runTestDynamic } from "./wptTestHarness.js";
import fs from "fs";
import path from "path";

const CWD = process.cwd();
const baseDir = `${CWD}/tests/wpt/wpt`;

const testFiles = [
  "async-iterator.any.js",
  "bad-strategies.any.js",
  "bad-underlying-sources.any.js",
  "cancel.any.js",
  "constructor.any.js",
  "count-queueing-strategy-integration.any.js",
  "default-reader.any.js",
  "floating-point-total-queue-size.any.js",
  "from.any.js",
  "garbage-collection.any.js",
  "general.any.js",
  "owning-type-message-port.any.js",
  // SKIP: VideoFrame support is pending
  // "owning-type-video-frame.any.js",
  "owning-type.any.js",
  "patched-global.any.js",
  "reentrant-strategies.any.js",
  "tee.any.js",
  "templated.any.js",
];

describe("readable-streams", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      const filePath = path.resolve(
        baseDir,
        "streams",
        "readable-streams",
        file
      );
      const sourceCode = fs.readFileSync(filePath, "utf8");
      runTestDynamic(sourceCode, done);
    });
  }
});
