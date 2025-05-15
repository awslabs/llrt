import encodings from "./encoding/resources/encodings.js";

import commonGc from "./common/gc.js";
import commonSubsetTests from "./common/subset-tests.js";

import resourcesIdlharness from "./resources/idlharness.js";
import resourcesTestharness from "./resources/testharness.js";

import encodingDecodingHelpers from "./encoding/resources/decoding-helpers.js";

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
