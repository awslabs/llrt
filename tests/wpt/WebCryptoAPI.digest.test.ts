import { runSuite, runTestDynamic } from "./WebCryptoAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [], {
  tentativeFiles: [
    "cshake.tentative.https.any.js",
    "sha3.tentative.https.any.js",
    "turboshake.tentative.https.any.js",
  ],
});
