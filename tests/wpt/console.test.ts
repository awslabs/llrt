import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./console.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "console-log-symbol.any.js", // Error: Test timed out after 5000ms
  "idlharness.any.js", // ReferenceError: idl_test is not defined
]);
