import { runSuite } from "./_harness-wpt.js";
import { runTestDynamic } from "./WebCryptoAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "pbkdf2.https.any.js", // Error: Test timed out after 5000ms
]);
