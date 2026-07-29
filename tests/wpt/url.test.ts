import { runSuite, runTestDynamic } from "./url.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "historical.any.js", // TypeError: cannot read property 'isWindow' of undefined
  "idlharness.any.js", // ReferenceError: idl_test is not defined
]);
