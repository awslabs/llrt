import { runTest } from "./runTest.js";

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
  "general.any.js",
  "properties.any.js",
  "reentrant-strategy.any.js",
  "start.any.js",
  "write.any.js",
];

describe.skip("writable-streams", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      runTest(require(`./streams/writable-streams/${file}`).default, done);
    });
  }
});
