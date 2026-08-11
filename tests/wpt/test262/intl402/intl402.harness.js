import {
  makeRunnerTest262 as makeRunner,
  runSuiteTest262 as runSuite,
} from "../../_harness-util.js";

const runTestDynamic = makeRunner({
  context: () => ({
    scripts: [
      "third_party/test262/harness/assert.js",
      "third_party/test262/harness/sta.js",
      "third_party/test262/harness/temporalHelpers.js",
    ],
  }),
  postSetup(context) {
    // Test262Error does not define a `name` property in the upstream
    // Test262 harness. LLRT's test runner uses `error.name` when
    // formatting errors, so provide the conventional name here.
    context.Test262Error.prototype.name = "Test262Error";
  },
});

export { runSuite, runTestDynamic };
