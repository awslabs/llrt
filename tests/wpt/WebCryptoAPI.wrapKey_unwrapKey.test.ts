import { runSuite, runTestDynamic } from "./WebCryptoAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  [
    "wrapKey_unwrapKey.https.any.js",
    ["[setup] Key import/export format must be 'jwk','raw','spki' or 'pkcs8'"], // 'raw-secret' is not supported.
  ],
]);
