import resourcesIdlharness from "./resources/idlharness.js";
import resourcesTestharness from "./resources/testharness.js";

import commonGc from "./common/gc.js";
import commonSubsetTests from "./common/subset-tests.js";

import encodings from "./encoding/resources/encodings.js";
import encodingDecodingHelpers from "./encoding/resources/decoding-helpers.js";

export const runTestDynamic = (testSource, done) => {
  const context = {
    createBuffer: (type, length) => new self[type](length),
    encodings_table: encodings,
    setTimeout: setTimeout,
    DOMException: DOMException,
    location: {},
  };

  resourcesIdlharness(context);
  resourcesTestharness(context);

  commonGc(context);
  commonSubsetTests(context);

  encodingDecodingHelpers(context);

  context.setup({
    explicit_done: true,
    debug: process.env.DEBUG !== undefined,
  });

  globalThis.gc = globalThis.__gc;

  context.add_completion_callback((tests, status, assertions) => {
    if (
      tests.filter(
        ({ name, status }) => !(name === "Loading data..." && status === 0)
      ).length === 0
    ) {
      done(new Error("No tests were executed!"));
    }
    const failure = tests.find((test) => test.status !== 0);
    if (failure) {
      const message = `[${failure.name}] ${failure.message || String(failure)}`;
      done(message);
      return;
    }
    done();
  });

  wrapTestSuite(testSource)(context);

  context.done();
};

function wrapTestSuite(sourceCode) {
  return new Function(
    "context",
    `
      with (context) {
        ${sourceCode}
      }
    `
  );
}
