import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./FileAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "Blob-constructor.any.js", // ReferenceError: promise_test is not defined
  "Blob-slice.any.js", // ReferenceError: promise_test is not defined
]);
