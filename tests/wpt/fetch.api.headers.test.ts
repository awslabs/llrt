import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./fetch.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "header-values-normalize.any.js", // TypeError: cannot read property 'isWorker' of undefined
  "header-values.any.js", // TypeError: cannot read property 'isWorker' of undefined
]);
