import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./WebCryptoAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "aes_gcm_256_iv.https.any.js", // NOTICE: Only 96-bit IVs are supported
]);
