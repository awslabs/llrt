import { makeRunner } from "./_harness-wpt.js";

export const runTestDynamic = makeRunner({
  context: () => ({ scripts: ["encoding/resources/encodings.js"] }),
});
