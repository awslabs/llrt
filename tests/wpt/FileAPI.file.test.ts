import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./FileAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "send-file-formdata-controls.any.js", // ReferenceError: promise_test is not defined
  "send-file-formdata-punctuation.any.js", // ReferenceError: promise_test is not defined
  "send-file-formdata-utf-8.any.js", // ReferenceError: promise_test is not defined
  "send-file-formdata.any.js", // ReferenceError: promise_test is not defined
]);
