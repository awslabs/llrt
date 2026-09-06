import { runSuite, runTestDynamic } from "./FileAPI.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  ["Blob-constructor.any.js", [/MessageChannel is not defined/]],
  [
    "Blob-constructor-detached-buffer.any.js",
    [/MessageChannel is not defined/],
  ],
]);
