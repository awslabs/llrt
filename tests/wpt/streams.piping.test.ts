import { runSuite, runTestDynamic } from "./streams.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "general-addition.any.js", // waiting on resolution of https://github.com/whatwg/streams/issues/1243.
]);
