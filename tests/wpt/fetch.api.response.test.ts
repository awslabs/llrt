import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./fetch.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "response-blob-realm.any.js", // requires Window
  "response-clone.any.js", // Error: Timeout after 5000ms
]);
