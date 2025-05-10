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

describe("readable-streams", () => {
  it("should pass async-iterator.any.js tests", (done) => {
    runTest(
      require("./streams/readable-streams/async-iterator.any.js").default,
      done
    );
  });

  it("should pass bad-strategies.any.js tests", (done) => {
    runTest(
      require("./streams/readable-streams/bad-strategies.any.js").default,
      done
    );
  });

  it("should pass bad-underlying-sources.any.js tests", (done) => {
    runTest(
      require("./streams/readable-streams/bad-underlying-sources.any.js")
        .default,
      done
    );
  });

  it("should pass cancel.any.js tests", (done) => {
    runTest(require("./streams/readable-streams/cancel.any.js").default, done);
  });

  it("should pass constructor.any.js tests", (done) => {
    runTest(
      require("./streams/readable-streams/constructor.any.js").default,
      done
    );
  });

  it("should pass count-queueing-strategy-integration.any.js tests", (done) => {
    runTest(
      require("./streams/readable-streams/count-queuing-strategy-integration.any.js")
        .default,
      done
    );
  });

  it("should pass default-reader.any.js tests", (done) => {
    runTest(
      require("./streams/readable-streams/default-reader.any.js").default,
      done
    );
  });

  it("should pass floating-point-total-queue-size.any.js tests", (done) => {
    runTest(
      require("./streams/readable-streams/floating-point-total-queue-size.any.js")
        .default,
      done
    );
  });

  it("should pass from.any.js tests", (done) => {
    runTest(require("./streams/readable-streams/from.any.js").default, done);
  });

  it("should pass garbage-collection.any.js tests", (done) => {
    runTest(
      require("./streams/readable-streams/garbage-collection.any.js").default,
      done
    );
  });

  it("should pass general.any.js tests", (done) => {
    runTest(require("./streams/readable-streams/general.any.js").default, done);
  });

  // needs owning type impl
  // it("should pass owning-type.any.js tests", (done) => {
  //   runTest(
  //     require("./streams/readable-streams/owning-type.any.js")
  //       .default,
  //     done
  //   );
  // });

  // needs handling of patched Promise.then fns
  // it("should pass patched-global.any.js tests", (done) => {
  //   runTest(
  //     require("./streams/readable-streams/patched-global.any.js")
  //       .default,
  //     done
  //   );
  // });

  it("should pass reentrant-strategies.any.js tests", (done) => {
    runTest(
      require("./streams/readable-streams/reentrant-strategies.any.js").default,
      done
    );
  });

  it("should pass tee.any.js tests", (done) => {
    runTest(require("./streams/readable-streams/tee.any.js").default, done);
  });

  it("should pass templated.any.js tests", (done) => {
    runTest(
      require("./streams/readable-streams/templated.any.js").default,
      done
    );
  });
});
