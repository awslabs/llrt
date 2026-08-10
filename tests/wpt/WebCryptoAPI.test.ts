import { runSuite, runTestDynamic } from "./WebCryptoAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "idlharness.https.any.js", // ReferenceError: idl_test is not defined
]);
