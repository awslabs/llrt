import { runTest } from "./runTest.js";

const testFiles = [
  "bad-buffers-and-views.any.js",
  "construct-byob-request.any.js",
  "enqueue-with-detached-buffer.any.js",
  "general.any.js",
  // "non-transferable-buffers.any.js", // requires WebAssembly (!?)
  "read-min.any.js",
  "respond-after-enqueue.any.js",
  "tee.any.js",
];

describe("readable-byte-streams", () => {
  for (const file of testFiles) {
    it(`should pass ${file} tests`, (done) => {
      runTest(require(`./streams/readable-byte-streams/${file}`).default, done);
    });
  }
});
