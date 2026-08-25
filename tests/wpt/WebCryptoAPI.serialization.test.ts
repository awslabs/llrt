import { runSuite, runTestDynamic } from "./WebCryptoAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [], {
  tentativeFiles: [
    "chacha20-poly1305.tentative.https.any.js",
    "hybridkem.tentative.https.window.js",
    "mldsa.tentative.https.any.js",
    "mlkem.tentative.https.any.js",
  ],
  filePattern: /\.(?:any|window)\.js$/,
});
