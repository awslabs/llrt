import { runTest } from "./runTest.js";
import fs from "fs";
import path from "path";

const CWD = process.cwd();
const baseDir = `${CWD}/tests/wpt`;

const testFiles = [
  "abort.any.js",
  "close-propagation-backward.any.js",
  "close-propagation-forward.any.js",
  "error-propagation-backward.any.js",
  "error-propagation-forward.any.js",
  "flow-control.any.js",
  // "general-addition.any.js", // waiting on resolution of https://github.com/whatwg/streams/issues/1243.
  "general.any.js",
  "multiple-propagation.any.js",
  "pipe-through.any.js",
  "then-interception.any.js",
  // "throwing-options.any.js", // requires TransformStream
  // "transform-streams.any.js", // requires TransformStream
];

describe("piping", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      runTest(require(`./streams/piping/${file}`).default, done);
    });
  }
});
