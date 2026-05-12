import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./streams.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "non-transferable-buffers.any.js", // SKIP: WebAssembly support is pending
  "general.any.js", // SKIP: hangs
  "templated.any.js", // SKIP: hangs
]);
