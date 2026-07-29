import { runSuite, runTestDynamic } from "./fetch.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "cache.https.any.js", // ReferenceError: caches is not defined
  "general.any.js", // Error: Timeout after 5000ms
]);
