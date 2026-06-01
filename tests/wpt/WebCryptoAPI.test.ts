import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./WebCryptoAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  /\.tentative\./,
  "getRandomValues.any.js", // It's Slowly...
  "idlharness.https.any.js", // ReferenceError: idl_test is not defined
]);
