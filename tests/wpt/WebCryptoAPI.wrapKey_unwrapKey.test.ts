import { runSuite, runTestDynamic } from "./WebCryptoAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  [
    "wrapKey_unwrapKey.https.any.js",
    /(?:using .* and AES-GCM|AES-GCM.*non-extractable)/, // Upstream vectors use a 128-bit IV; only 96-bit IVs are supported.
  ],
]);
