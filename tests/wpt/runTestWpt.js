import subsetTests from "./common/subset-tests.js";
import gc from "./common/gc.js";
import idlharness from "./resources/idlharness.js";
import testharness from "./resources/testharness.js";
import encodings from "./encoding/resources/encodings.js";
import recordingStreams from "./streams/resources/recording-streams.js";
import testUtils from "./streams/resources/test-utils.js";
import rsUtils from "./streams/resources/rs-utils.js";
import rsTestTemplates from "./streams/resources/rs-test-templates.js";

export const runTestWpt = (testSource, done) => {
  const context = {
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
    location: {},
  };

  idlharness(context);
  gc(context);
  testharness(context);
  subsetTests(context);
  recordingStreams(context);
  testUtils(context);
  rsUtils(context);
  rsTestTemplates(context);

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
    done(failure);
  });

  const testFunction = wrapTestSource(testSource);
  testFunction(context);

  context.done();
};

function wrapTestSource(sourceCode) {
  return new Function("context", `
      with (context) {
        ${sourceCode}
      }
    `);
}