import { runSuite, runTestDynamic } from "./WebCryptoAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "historical.any.js", // Non-secure Window/Worker context is not applicable to LLRT
  "idlharness.https.any.js", // ReferenceError: idl_test is not defined
]);
