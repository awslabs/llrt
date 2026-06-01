import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./streams.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "owning-type-video-frame.any.js", // SKIP: VideoFrame support is pending
  "owning-type-message-port.any.js", // needs MessageChannel (browser-only)
  "templated.any.js", // SKIP: hangs on async-iterator / default-reader tests
  "default-reader.any.js", // SKIP: hangs
]);
