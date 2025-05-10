import { runTest } from "./runTest.js";
import fs from "fs";
import path from "path";

const CWD = process.cwd();
const baseDir = `${CWD}/tests/wpt`;

const testFiles = [
  "async-iterator.any.js",
  "bad-strategies.any.js",
  "bad-underlying-sources.any.js",
  "cancel.any.js",
  "constructor.any.js",
  // "count-queueing-strategy-integration.any.js",
  "default-reader.any.js",
  "floating-point-total-queue-size.any.js",
  "from.any.js",
  "garbage-collection.any.js",
  "general.any.js",
  // "owning-type.any.js", // needs owning type impl
  // "patched-global.any.js", // needs handling of patched Promise.then fns
  "reentrant-strategies.any.js",
  "tee.any.js",
  "templated.any.js",
];

describe("readable-streams", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      runTest(require(`./streams/readable-streams/${file}`).default, done);
    });
  }
});
