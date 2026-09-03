import { runSuite, runTestDynamic } from "./WebCryptoAPI.harness.js";

runSuite(
  import.meta.url,
  runTestDynamic,
  [["getPublicKey.tentative.https.any.js", /(?:Ed448|X448)/]],
  {
    subDir: "WebCryptoAPI",
    filePattern: /^getPublicKey\.tentative\.https\.any\.js$/,
    tentativeFiles: ["getPublicKey.tentative.https.any.js"],
  }
);
