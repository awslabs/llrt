import { runSuite, runTestDynamic } from "./WebCryptoAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [], {
  subDir: "WebCryptoAPI",
  filePattern: /^supports(?:-modern)?\.tentative\.https\.any\.js$/,
  tentativeFiles: [
    "supports-modern.tentative.https.any.js",
    "supports.tentative.https.any.js",
  ],
});
