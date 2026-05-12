import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./fetch.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "request-cache-default-conditional.any.js", // ReferenceError: promise_test is not defined
  "request-cache-default.any.js", // ReferenceError: promise_test is not defined
  "request-cache-force-cache.any.js", // ReferenceError: promise_test is not defined
  "request-cache-no-cache.any.js", // ReferenceError: promise_test is not defined
  "request-cache-no-store.any.js", // ReferenceError: promise_test is not defined
  "request-cache-only-if-cached.any.js", // ReferenceError: promise_test is not defined
  "request-cache-reload.any.js", // ReferenceError: promise_test is not defined
]);
