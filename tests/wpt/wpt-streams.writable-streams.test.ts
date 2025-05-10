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

describe.skip("writable-streams", () => {
  it("should pass aborting.any.js tests", (done) => {
    runTest(
      require("./streams/writable-streams/aborting.any.js").default,
      done
    );
  });

  it("should pass bad-strategies.any.js tests", (done) => {
    runTest(
      require("./streams/writable-streams/bad-strategies.any.js").default,
      done
    );
  });

  it("should pass bad-underlying-sinks.any.js tests", (done) => {
    runTest(
      require("./streams/writable-streams/bad-underlying-sinks.any.js").default,
      done
    );
  });

  it("should pass byte-length-queuing-strategy.any.js tests", (done) => {
    runTest(
      require("./streams/writable-streams/byte-length-queuing-strategy.any.js")
        .default,
      done
    );
  });

  it("should pass close.any.js tests", (done) => {
    runTest(require("./streams/writable-streams/close.any.js").default, done);
  });

  it("should pass constructor.any.js tests", (done) => {
    runTest(
      require("./streams/writable-streams/constructor.any.js").default,
      done
    );
  });

  it("should pass count-queuing-strategy.any.js tests", (done) => {
    runTest(
      require("./streams/writable-streams/count-queuing-strategy.any.js")
        .default,
      done
    );
  });

  it("should pass error.any.js tests", (done) => {
    runTest(require("./streams/writable-streams/error.any.js").default, done);
  });

  it("should pass floating-point-total-queue-size.any.js tests", (done) => {
    runTest(
      require("./streams/writable-streams/floating-point-total-queue-size.any.js")
        .default,
      done
    );
  });

  it("should pass general.any.js tests", (done) => {
    runTest(require("./streams/writable-streams/general.any.js").default, done);
  });

  it("should pass properties.any.js tests", (done) => {
    runTest(
      require("./streams/writable-streams/properties.any.js").default,
      done
    );
  });

  it("should pass reentrant-strategy.any.js tests", (done) => {
    runTest(
      require("./streams/writable-streams/reentrant-strategy.any.js").default,
      done
    );
  });

  it("should pass start.any.js tests", (done) => {
    runTest(require("./streams/writable-streams/start.any.js").default, done);
  });

  it("should pass write.any.js tests", (done) => {
    runTest(require("./streams/writable-streams/write.any.js").default, done);
  });
});
