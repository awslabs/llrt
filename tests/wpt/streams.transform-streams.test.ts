import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./streams.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "backpressure.any.js", // Error: Timeout after 5000ms
  "cancel.any.js", // Error: Timeout after 5000ms
  "errors.any.js", // Error: Timeout after 5000ms
  "flush.any.js", // Error: Timeout after 5000ms
  "general.any.js", // Error: Timeout after 5000ms
  "reentrant-strategies.any.js", // Error: Timeout after 5000ms
]);
