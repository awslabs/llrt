import encodings from "./encoding/resources/encodings.js";

import commonGc from "./common/gc.js";
import commonSubsetTests from "./common/subset-tests.js";

import resourcesIdlharness from "./resources/idlharness.js";
import resourcesTestharness from "./resources/testharness.js";

import encodingDecodingHelpers from "./encoding/resources/decoding-helpers.js";

export const runTest = (test, done) => {
  //
  // Set up the test harness
  //

  // Create a new test context
  const context = {
    // The test harness uses common/sab.js which uses WebAssembly which doesn't
    // work, so we can just create buffers the usual way
    createBuffer: (type, length) => new self[type](length),
    encodings_table: encodings,
    setTimeout: setTimeout,
    DOMException: DOMException,
    // Some tests require location to be defined
    location: {},
  };

  // Initialize the test harness in the context
  commonGc(context);
  commonSubsetTests(context);

  resourcesIdlharness(context);
  resourcesTestharness(context);

  encodingDecodingHelpers(context);

  // Configure the test harness
  context.setup({
    explicit_done: true,
    debug: process.env.DEBUG !== undefined,
  });

  globalThis.gc = globalThis.__gc;

  context.add_completion_callback((tests, status, assertions) => {
    // Check that tests were actually executed not including the optional step
    // that loads test data
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

  test(context);

  context.done();
};

export const runTestDynamic = (testSource, done) => {
  const context = {
    createBuffer: (type, length) => new self[type](length),
    encodings_table: encodings,
    setTimeout: setTimeout,
    DOMException: DOMException,
    location: {},
  };

  commonGc(context);
  commonSubsetTests(context);

  resourcesIdlharness(context);
  resourcesTestharness(context);

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
