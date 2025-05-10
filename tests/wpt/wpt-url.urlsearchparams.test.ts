import idlharness from "./resources/idlharness.js";
import testharness from "./resources/testharness.js";
import subsetTests from "./common/subset-tests.js";
import encodings from "./encoding/resources/encodings.js";

const runTest = (test, done) => {
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
  idlharness(context);
  testharness(context);
  subsetTests(context);

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
    done(failure);
  });

  test(context);

  context.done();
};

describe("urlsearchparams", () => {
  it("should pass urlsearchparams-append.any.js tests", (done) => {
    runTest(require("./url/urlsearchparams-append.any.js").default, done);
  });

  it("should pass urlsearchparams-constructor.any.js tests", (done) => {
    runTest(require("./url/urlsearchparams-constructor.any.js").default, done);
  });

  it("should pass urlsearchparams-delete.any.js tests", (done) => {
    runTest(require("./url/urlsearchparams-delete.any.js").default, done);
  });

  it("should pass urlsearchparams-foreach.any.js tests", (done) => {
    runTest(require("./url/urlsearchparams-foreach.any.js").default, done);
  });

  it("should pass urlsearchparams-get.any.js tests", (done) => {
    runTest(require("./url/urlsearchparams-get.any.js").default, done);
  });

  it("should pass urlsearchparams-getall.any.js tests", (done) => {
    runTest(require("./url/urlsearchparams-getall.any.js").default, done);
  });

  it("should pass urlsearchparams-has.any.js tests", (done) => {
    runTest(require("./url/urlsearchparams-has.any.js").default, done);
  });

  it("should pass urlsearchparams-set.any.js tests", (done) => {
    runTest(require("./url/urlsearchparams-set.any.js").default, done);
  });

  it("should pass urlsearchparams-size.any.js tests", (done) => {
    runTest(require("./url/urlsearchparams-size.any.js").default, done);
  });

  it("should pass urlsearchparams-sort.any.js tests", (done) => {
    runTest(require("./url/urlsearchparams-sort.any.js").default, done);
  });

  it("should pass urlsearchparams-stringifier.any.js tests", (done) => {
    runTest(require("./url/urlsearchparams-stringifier.any.js").default, done);
  });
});
