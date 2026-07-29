import {
  makeRunnerWPT as makeRunner,
  runSuiteWPT as runSuite,
} from "./_harness-util.js";

const runTestDynamic = makeRunner({
  context: () => ({
    scripts: [
      "encoding/resources/encodings.js",
      "FileAPI/support/Blob.js",
      "FileAPI/support/send-file-formdata-helper.js",
    ],
  }),
});

export { runSuite, runTestDynamic };
