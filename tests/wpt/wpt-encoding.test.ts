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

describe("encoding", () => {
  it("should pass api-basics.any.js", (done) => {
    runTest(require("./encoding/api-basics.any.js").default, done);
  });

  it("should pass api-invalid-label.any.js", (done) => {
    runTest(require("./encoding/api-invalid-label.any.js").default, done);
  });

  it("should pass api-replacement-encodings.any.js", (done) => {
    runTest(
      require("./encoding/api-replacement-encodings.any.js").default,
      done
    );
  });

  it("should pass api-surrogates-utf8.any.js", (done) => {
    runTest(require("./encoding/api-surrogates-utf8.any.js").default, done);
  });

  it("should pass encodeInto.any.js", (done) => {
    runTest(require("./encoding/encodeInto.any.js").default, done);
  });

  // Current support is utf8 and utf16le
  // it("should pass iso-2022-jp-decoder.any.js", (done) => {
  //   runTest(
  //     require("./encoding/iso-2022-jp-decoder.any.js")
  //       .default,
  //     done
  //   );
  // });

  // Requires XMLHTTPRequest which is not defined
  // it("should pass replacement-encodings.any.js", (done) => {
  //   runTest(
  //     require("./encoding/replacement-encodings.any.js")
  //       .default,
  //     done
  //   );
  // });

  // Requires XMLHTTPRequest which is not defined
  // it("should pass unsupported-encodings.any.js", (done) => {
  //   runTest(
  //     require("./encoding/unsupported-encodings.any.js")
  //       .default,
  //     done
  //   );
  // });
});
