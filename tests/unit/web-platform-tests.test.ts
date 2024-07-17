import idlharness from "./web-platform-tests/resources/idlharness.js";
import testharness from "./web-platform-tests/resources/testharness.js";
import subsetTests from "./web-platform-tests/common/subset-tests.js";
import encodings from "./web-platform-tests/encoding/resources/encodings.js";

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
          data = require("./web-platform-tests/url/resources/urltestdata.json");
          break;
        case "resources/setters_tests.json":
          data = require("./web-platform-tests/url/resources/setters_tests.json");
          break;
        default:
          throw new Error(`Cannot fetch URL: ${url}`);
      }
      return Promise.resolve({
        json: () => Promise.resolve(data),
      });
    },
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

describe("web-platform-tests", () => {
  describe("encoding", () => {
    it("should pass api-basics.any.js", (done) => {
      runTest(
        require("./web-platform-tests/encoding/api-basics.any.js").default,
        done
      );
    });

    it("should pass api-invalid-label.any.js", (done) => {
      runTest(
        require("./web-platform-tests/encoding/api-invalid-label.any.js")
          .default,
        done
      );
    });

    it("should pass api-replacement-encodings.any.js", (done) => {
      runTest(
        require("./web-platform-tests/encoding/api-replacement-encodings.any.js")
          .default,
        done
      );
    });

    it("should pass api-surrogates-utf8.any.js", (done) => {
      runTest(
        require("./web-platform-tests/encoding/api-surrogates-utf8.any.js")
          .default,
        done
      );
    });

    it("should pass encodeInto.any.js", (done) => {
      runTest(
        require("./web-platform-tests/encoding/encodeInto.any.js").default,
        done
      );
    });

    // Current support is utf8 and utf16le
    // it("should pass iso-2022-jp-decoder.any.js", (done) => {
    //   runTest(
    //     require("./web-platform-tests/encoding/iso-2022-jp-decoder.any.js")
    //       .default,
    //     done
    //   );
    // });

    // Requires XMLHTTPRequest which is not defined
    // it("should pass replacement-encodings.any.js", (done) => {
    //   runTest(
    //     require("./web-platform-tests/encoding/replacement-encodings.any.js")
    //       .default,
    //     done
    //   );
    // });

    it("should pass textdecoder-arguments.any.js", (done) => {
      runTest(
        require("./web-platform-tests/encoding/textdecoder-arguments.any.js")
          .default,
        done
      );
    });

    it("should pass textdecoder-byte-order-marks.any.js", (done) => {
      runTest(
        require("./web-platform-tests/encoding/textdecoder-byte-order-marks.any.js")
          .default,
        done
      );
    });

    // stream option not implemented
    // it("should pass textdecoder-copy.any.js", (done) => {
    //   runTest(
    //     require("./web-platform-tests/encoding/textdecoder-copy.any.js").default,
    //     done
    //   );
    // });

    it("should pass textdecoder-eof.any.js", (done) => {
      runTest(
        require("./web-platform-tests/encoding/textdecoder-eof.any.js").default,
        done
      );
    });

    // Current support is utf8 and utf16le
    // it("should pass textdecoder-fatal-single-byte.any.js", (done) => {
    //   runTest(
    //     require("./web-platform-tests/encoding/textdecoder-fatal-single-byte.any.js")
    //       .default,
    //     done
    //   );
    // });

    it("should pass textdecoder-fatal-streaming.any.js", (done) => {
      runTest(
        require("./web-platform-tests/encoding/textdecoder-fatal-streaming.any.js")
          .default,
        done
      );
    });

    it("should pass textdecoder-fatal.any.js", (done) => {
      runTest(
        require("./web-platform-tests/encoding/textdecoder-fatal.any.js")
          .default,
        done
      );
    });

    it("should pass textdecoder-ignorebom.any.js", (done) => {
      runTest(
        require("./web-platform-tests/encoding/textdecoder-ignorebom.any.js")
          .default,
        done
      );
    });

    // Not implemented
    // it("should pass textdecoder-labels.any.js", (done) => {
    //   runTest(
    //     require("./web-platform-tests/encoding/textdecoder-labels.any.js")
    //       .default,
    //     done
    //   );
    // });

    // stream option not implemented
    // it("should pass textdecoder-streaming.any.js", (done) => {
    //   runTest(
    //     require("./web-platform-tests/encoding/textdecoder-streaming.any.js")
    //       .default,
    //     done
    //   );
    // });

    it("should pass textdecoder-utf16-surrogates.any.js", (done) => {
      runTest(
        require("./web-platform-tests/encoding/textdecoder-utf16-surrogates.any.js")
          .default,
        done
      );
    });

    it("should pass textencoder-constructor-non-utf.any.js", (done) => {
      runTest(
        require("./web-platform-tests/encoding/textencoder-constructor-non-utf.any.js")
          .default,
        done
      );
    });

    // it("should pass textencoder-utf16-surrogates.any.js", (done) => {
    //   runTest(
    //     require("./web-platform-tests/encoding/textencoder-utf16-surrogates.any.js")
    //       .default,
    //     done
    //   );
    // });

    // Requires XMLHTTPRequest which is not defined
    // it("should pass unsupported-encodings.any.js", (done) => {
    //   runTest(
    //     require("./web-platform-tests/encoding/unsupported-encodings.any.js")
    //       .default,
    //     done
    //   );
    // });
  });

  describe("url", () => {
    // Not testing these edge cases
    // require("./web-platform-tests/url/historical.any.js");
    // Request.formData() not supported
    // require("./web-platform-tests/url/urlencoded-parser.any.js");

    it("should pass url-constructor.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/url-constructor.any.js").default,
        done
      );
    });

    it("should pass url-origin.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/url-origin.any.js").default,
        done
      );
    });

    it("should pass url-searchparams.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/url-searchparams.any.js").default,
        done
      );
    });

    it("should pass url-setters.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/url-setters.any.js").default,
        done
      );
    });

    it("should pass url-setters-stripping.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/url-setters-stripping.any.js")
          .default,
        done
      );
    });

    it("should pass url-statics-canparse.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/url-statics-canparse.any.js").default,
        done
      );
    });

    it("should pass url-statics-parse.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/url-statics-parse.any.js").default,
        done
      );
    });

    it("should pass url-tojson.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/url-tojson.any.js").default,
        done
      );
    });

    it("should pass urlsearchparams-append.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/urlsearchparams-append.any.js")
          .default,
        done
      );
    });

    it("should pass urlsearchparams-constructor.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/urlsearchparams-constructor.any.js")
          .default,
        done
      );
    });

    it("should pass urlsearchparams-delete.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/urlsearchparams-delete.any.js")
          .default,
        done
      );
    });

    it("should pass urlsearchparams-foreach.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/urlsearchparams-foreach.any.js")
          .default,
        done
      );
    });

    it("should pass urlsearchparams-get.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/urlsearchparams-get.any.js").default,
        done
      );
    });

    it("should pass urlsearchparams-getall.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/urlsearchparams-getall.any.js")
          .default,
        done
      );
    });

    it("should pass urlsearchparams-has.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/urlsearchparams-has.any.js").default,
        done
      );
    });

    it("should pass urlsearchparams-set.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/urlsearchparams-set.any.js").default,
        done
      );
    });

    it("should pass urlsearchparams-size.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/urlsearchparams-size.any.js").default,
        done
      );
    });

    it("should pass urlsearchparams-sort.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/urlsearchparams-sort.any.js").default,
        done
      );
    });

    it("should pass urlsearchparams-stringifier.any.js tests", (done) => {
      runTest(
        require("./web-platform-tests/url/urlsearchparams-stringifier.any.js")
          .default,
        done
      );
    });
  });
});
