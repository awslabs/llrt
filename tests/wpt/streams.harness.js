import { makeRunner } from "./_harness-util.js";

export const runTestDynamic = makeRunner({
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
