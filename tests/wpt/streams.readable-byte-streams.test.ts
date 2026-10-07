import { runSuite, runTestDynamic } from "./streams.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  ["non-transferable-buffers.any.js", [/WebAssembly is not defined/]],
]);
