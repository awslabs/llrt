import { runSuite, runTestDynamic } from "./hr-time.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "idlharness.any.js", // ReferenceError: idl_test is not defined
]);
