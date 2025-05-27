import resourcesIdlharness from "./resources/idlharness.js";
import resourcesTestharness from "./resources/testharness.js";

import commonGc from "./common/gc.js";
import commonSubsetTests from "./common/subset-tests.js";

import encodings from "./encoding/resources/encodings.js";

import fetchKeepaliveHelper from "./fetch/api/resources/keepalive-helper.js";
import fetchKeepaliveWorker from "./fetch/api/resources/keepalive-worker.js";
import fetchSwInterceptAbort from "./fetch/api/resources/sw-intercept-abort.js";
import fetchSwIntercept from "./fetch/api/resources/sw-intercept.js";
import fetchUtils from "./fetch/api/resources/utils.js";
import fetchRequestRequestCache from "./fetch/api/request/request-cache.js";

export const runTestDynamic = (testSource, baseDir, done) => {
  globalThis._fetch = globalThis.fetch;

  const context = {
    createBuffer: (type, length) => new self[type](length),
    encodings_table: encodings,
    setTimeout: setTimeout,
    DOMException: DOMException,
    location: {},
    RESOURCES_DIR: "",

    fetch: (url, option) => {
      let data;
      switch (url) {
        case "../cors/resources/not-cors-safelisted.json":
          data = require(
            baseDir + "/fetch/api/cors/resources/not-cors-safelisted.json"
          );
          break;
        default:
          return _fetch(url, option);
      }
      return Promise.resolve({
        json: () => Promise.resolve(data),
      });
    },
  };

  resourcesIdlharness(context);
  resourcesTestharness(context);

  commonGc(context);
  commonSubsetTests(context);

  fetchKeepaliveHelper(context);
  // fetchKeepaliveWorker(context);
  // fetchSwInterceptAbort(context);
  // fetchSwIntercept(context);
  fetchUtils(context);
  fetchRequestRequestCache(context);

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
