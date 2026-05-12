import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./fetch.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "cache.https.any.js", // ReferenceError: caches is not defined
  "general.any.js", // Error: Timeout after 5000ms
]);
