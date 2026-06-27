import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./fetch.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "request-forbidden-headers.any.js", // ReferenceError: promise_test is not defined
  "request-upload.h2.any.js",
  "scheme-blob.sub.any.js", // TypeError: not a function
  "error-after-response.any.js", // hangs: response reader doesn't abort on network error
  "keepalive.any.js", // needs document global (browser-only)
]);
