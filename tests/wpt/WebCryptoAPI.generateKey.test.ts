import { runSuite, runTestDynamic } from "./WebCryptoAPI.harness.js";

runSuite(
  import.meta.url,
  runTestDynamic,
  [
    "successes_RSA-OAEP.https.any.js", // Error: Test timed out after 5000ms
    "successes_RSA-PSS.https.any.js", // Error: Test timed out after 5000ms
    "successes_RSASSA-PKCS1-v1_5.https.any.js", // Error: Test timed out after 5000ms
  ],
  {
    tentativeFiles: [
      "failures_chacha20_poly1305.tentative.https.any.js",
      "failures_Hybrid-KEM.tentative.https.any.js",
      "failures_ML-DSA.tentative.https.any.js",
      "failures_ML-KEM.tentative.https.any.js",
      "successes_chacha20_poly1305.tentative.https.any.js",
      "successes_Hybrid-KEM.tentative.https.any.js",
      "successes_ML-DSA.tentative.https.any.js",
      "successes_ML-KEM.tentative.https.any.js",
    ],
  }
);
