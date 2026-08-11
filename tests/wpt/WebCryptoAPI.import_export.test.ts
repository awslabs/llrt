import { runSuite, runTestDynamic } from "./WebCryptoAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  // Enable the remaining import/export WPTs separately as support lands.
  /^(?!rsa_importKey\.https\.any\.js$)/,
]);
