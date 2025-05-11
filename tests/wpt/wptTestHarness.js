import encodings from "./encoding/resources/encodings.js";

import commonGc from "./common/gc.js";
import commonSubsetTests from "./common/subset-tests.js";

import resourcesIdlharness from "./resources/idlharness.js";
import resourcesTestharness from "./resources/testharness.js";

import encodingDecodingHelpers from "./encoding/resources/decoding-helpers.js";

import streamsRecordingStreams from "./streams/resources/recording-streams.js";
import streamsRsTestTemplates from "./streams/resources/rs-test-templates.js";
import streamsRsUtils from "./streams/resources/rs-utils.js";
import streamsTestUtils from "./streams/resources/test-utils.js";

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
    fetch: (url) => {
      let data;
      switch (url) {
        case "resources/urltestdata.json":
          data = require("./url/resources/urltestdata.json");
          break;
        case "resources/setters_tests.json":
          data = require("./url/resources/setters_tests.json");
          break;
        default:
          throw new Error(`Cannot fetch URL: ${url}`);
      }
      return Promise.resolve({
        json: () => Promise.resolve(data),
      });
    },
    setTimeout: setTimeout,
    DOMException: DOMException,
    // Some tests require location to be defined
    location: {},
  };

  // Initialize the test harness in the context
  resourcesTestharness(context);

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
    fetch: (url) => {
      let data;
      switch (url) {
        case "resources/urltestdata-javascript-only.json":
          data = require("./url/resources/urltestdata-javascript-only.json");
          break;
        case "resources/urltestdata.json":
          data = require("./url/resources/urltestdata.json");
          break;
        case "resources/setters_tests.json":
          data = require("./url/resources/setters_tests.json");
          break;
        default:
          throw new Error(`Cannot fetch URL: ${url}`);
      }
      return Promise.resolve({
        json: () => Promise.resolve(data),
      });
    },
    setTimeout: setTimeout,
    DOMException: DOMException,
    location: {},
  };

  commonGc(context);
  commonSubsetTests(context);

  resourcesIdlharness(context);
  resourcesTestharness(context);

  encodingDecodingHelpers(context);

  streamsRecordingStreams(context);
  streamsRsTestTemplates(context);
  streamsRsUtils(context);
  streamsTestUtils(context);

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
