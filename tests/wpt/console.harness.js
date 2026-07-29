import {
  makeRunnerWPT as makeRunner,
  runSuiteWPT as runSuite,
} from "./_harness-util.js";

const runTestDynamic = makeRunner({
  context: () => ({ scripts: ["encoding/resources/encodings.js"] }),
});

export { runSuite, runTestDynamic };
