import { makeRunner } from "./_harness-util.js";

export const runTestDynamic = makeRunner({
  context: () => ({
    scripts: [
      "encoding/resources/encodings.js",
      "FileAPI/support/Blob.js",
      "FileAPI/support/send-file-formdata-helper.js",
    ],
  }),
});
