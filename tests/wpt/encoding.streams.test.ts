import { runSuite, runTestDynamic } from "./encoding.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  ["decode-attributes.any.js", [/encoding is not supported/]],
  ["decode-non-utf8.any.js", [/encoding is not supported/]],
  ["decode-utf8.any.js", [/MessageChannel is not defined/]],
]);
