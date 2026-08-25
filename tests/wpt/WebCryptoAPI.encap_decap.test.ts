import { runSuite, runTestDynamic } from "./WebCryptoAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [], {
  tentativeFiles: [
    "encap_decap_bits.tentative.https.any.js",
    "encap_decap_keys.tentative.https.any.js",
  ],
});
