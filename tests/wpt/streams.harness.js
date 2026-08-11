import {
  makeRunnerWPT as makeRunner,
  runSuiteWPT as runSuite,
} from "./_harness-util.js";

const runTestDynamic = makeRunner({
  context: () => ({
    scripts: [
      "encoding/resources/encodings.js",
      "streams/resources/recording-streams.js",
      "streams/resources/rs-test-templates.js",
      "streams/resources/rs-utils.js",
      "streams/resources/test-utils.js",
    ],
  }),
});

export { runSuite, runTestDynamic };
