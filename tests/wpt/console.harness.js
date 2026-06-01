import { makeRunner } from "./_harness-util.js";

export const runTestDynamic = makeRunner({
  context: () => ({ scripts: ["encoding/resources/encodings.js"] }),
});
