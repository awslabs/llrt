import { runSuite, runTestDynamic } from "./streams.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "idlharness.any.js", // ReferenceError: idl_test is not defined
]);
