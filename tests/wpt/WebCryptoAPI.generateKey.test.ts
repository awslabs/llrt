import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./WebCryptoAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "successes_RSA-OAEP.https.any.js", // Error: Test timed out after 5000ms
  "successes_RSA-PSS.https.any.js", // Error: Test timed out after 5000ms
  "successes_RSASSA-PKCS1-v1_5.https.any.js", // Error: Test timed out after 5000ms
]);
