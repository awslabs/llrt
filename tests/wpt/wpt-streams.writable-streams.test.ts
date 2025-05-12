import { runTestDynamic } from "./wptTestHarness.js";
import fs from "fs";
import path from "path";

const CWD = process.cwd();
const baseDir = `${CWD}/tests/wpt/wpt`;

const testFiles = [
  "aborting.any.js",
  "bad-strategies.any.js",
  "bad-underlying-sinks.any.js",
  "byte-length-queuing-strategy.any.js",
  "close.any.js",
  "constructor.any.js",
  "count-queuing-strategy.any.js",
  "error.any.js",
  "floating-point-total-queue-size.any.js",
  "garbage-collection.any.js",
  "general.any.js",
  "properties.any.js",
  "reentrant-strategy.any.js",
  "start.any.js",
  "write.any.js",
];

describe.skip("writable-streams", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      const filePath = path.resolve(
        baseDir,
        "streams",
        "writable-streams",
        file
      );
      const sourceCode = fs.readFileSync(filePath, "utf8");
      runTestDynamic(sourceCode, done);
    });
  }
});
