import { runSuite, runTestDynamic } from "./WebCryptoAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [], {
  tentativeFiles: [
    "ChaCha20-Poly1305_importKey.tentative.https.any.js",
    "Hybrid-KEM_importKey.tentative.https.any.js",
    "ML-DSA_importKey.tentative.https.any.js",
    "ML-KEM_importKey.tentative.https.any.js",
    "raw_format_aliases.tentative.https.any.js",
  ],
});
