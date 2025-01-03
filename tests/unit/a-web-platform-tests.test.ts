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

    it("should pass textdecoder-labels.any.js", (done) => {
      runTest(
        require("./web-platform-tests/encoding/textdecoder-labels.any.js")
          .default,
        done
      );
    });

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

  describe("streams", () => {
    describe("readable-streams", () => {
      it("should pass async-iterator.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-streams/async-iterator.any.js")
            .default,
          done
        );
      });

      it("should pass bad-strategies.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-streams/bad-strategies.any.js")
            .default,
          done
        );
      });

      it("should pass bad-underlying-sources.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-streams/bad-underlying-sources.any.js")
            .default,
          done
        );
      });

      it("should pass cancel.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-streams/cancel.any.js")
            .default,
          done
        );
      });

      it("should pass constructor.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-streams/constructor.any.js")
            .default,
          done
        );
      });

      it("should pass count-queueing-strategy-integration.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-streams/count-queuing-strategy-integration.any.js")
            .default,
          done
        );
      });

      it("should pass default-reader.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-streams/default-reader.any.js")
            .default,
          done
        );
      });

      it("should pass floating-point-total-queue-size.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-streams/floating-point-total-queue-size.any.js")
            .default,
          done
        );
      });

      it("should pass from.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-streams/from.any.js")
            .default,
          done
        );
      });

      it("should pass garbage-collection.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-streams/garbage-collection.any.js")
            .default,
          done
        );
      });

      it("should pass general.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-streams/general.any.js")
            .default,
          done
        );
      });

      // needs owning type impl
      // it("should pass owning-type.any.js tests", (done) => {
      //   runTest(
      //     require("./web-platform-tests/streams/readable-streams/owning-type.any.js")
      //       .default,
      //     done
      //   );
      // });

      // needs handling of patched Promise.then fns
      // it("should pass patched-global.any.js tests", (done) => {
      //   runTest(
      //     require("./web-platform-tests/streams/readable-streams/patched-global.any.js")
      //       .default,
      //     done
      //   );
      // });

      it("should pass reentrant-strategies.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-streams/reentrant-strategies.any.js")
            .default,
          done
        );
      });

      it("should pass tee.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-streams/tee.any.js")
            .default,
          done
        );
      });

      it("should pass templated.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-streams/templated.any.js")
            .default,
          done
        );
      });
    });

    describe("readable-byte-streams", () => {
      it("should pass bad-buffers-and-views.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-byte-streams/bad-buffers-and-views.any.js")
            .default,
          done
        );
      });

      it("should pass construct-byob-request.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-byte-streams/construct-byob-request.any.js")
            .default,
          done
        );
      });

      it("should pass enqueue-with-detached-buffer.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-byte-streams/enqueue-with-detached-buffer.any.js")
            .default,
          done
        );
      });

      it("should pass general.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-byte-streams/general.any.js")
            .default,
          done
        );
      });

      // requires WebAssembly (!?)
      // it("should pass non-transferable-buffers.any.js tests", (done) => {
      //   runTest(
      //     require("./web-platform-tests/streams/readable-byte-streams/non-transferable-buffers.any.js")
      //       .default,
      //     done
      //   );
      // });

      it("should pass read-min.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-byte-streams/read-min.any.js")
            .default,
          done
        );
      });

      it("should pass respond-after-enqueue.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-byte-streams/respond-after-enqueue.any.js")
            .default,
          done
        );
      });

      it("should pass tee.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/readable-byte-streams/tee.any.js")
            .default,
          done
        );
      });
    });

    describe("writable-streams", () => {
      it("should pass aborting.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/writable-streams/aborting.any.js")
            .default,
          done
        );
      });

      it("should pass bad-strategies.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/writable-streams/bad-strategies.any.js")
            .default,
          done
        );
      });

      it("should pass bad-underlying-sinks.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/writable-streams/bad-underlying-sinks.any.js")
            .default,
          done
        );
      });

      it("should pass byte-length-queuing-strategy.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/writable-streams/byte-length-queuing-strategy.any.js")
            .default,
          done
        );
      });

      it("should pass close.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/writable-streams/close.any.js")
            .default,
          done
        );
      });

      it("should pass constructor.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/writable-streams/constructor.any.js")
            .default,
          done
        );
      });

      it("should pass count-queuing-strategy.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/writable-streams/count-queuing-strategy.any.js")
            .default,
          done
        );
      });

      it("should pass error.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/writable-streams/error.any.js")
            .default,
          done
        );
      });

      it("should pass floating-point-total-queue-size.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/writable-streams/floating-point-total-queue-size.any.js")
            .default,
          done
        );
      });

      it("should pass general.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/writable-streams/general.any.js")
            .default,
          done
        );
      });

      it("should pass properties.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/writable-streams/properties.any.js")
            .default,
          done
        );
      });

      it("should pass reentrant-strategy.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/writable-streams/reentrant-strategy.any.js")
            .default,
          done
        );
      });

      it("should pass start.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/writable-streams/start.any.js")
            .default,
          done
        );
      });

      it("should pass write.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/writable-streams/write.any.js")
            .default,
          done
        );
      });
    });

    describe("piping", () => {
      it("should pass abort.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/piping/abort.any.js").default,
          done
        );
      });

      it("should pass close-propagation-backward.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/piping/close-propagation-backward.any.js")
            .default,
          done
        );
      });

      it("should pass close-propagation-forward.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/piping/close-propagation-forward.any.js")
            .default,
          done
        );
      });

      it("should pass error-propagation-backward.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/piping/error-propagation-backward.any.js")
            .default,
          done
        );
      });

      it("should pass error-propagation-forward.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/piping/error-propagation-forward.any.js")
            .default,
          done
        );
      });

      it("should pass flow-control.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/piping/flow-control.any.js")
            .default,
          done
        );
      });

      // waiting on resolution of https://github.com/whatwg/streams/issues/1243.
      // it("should pass general-addition.any.js tests", (done) => {
      //   runTest(
      //     require("./web-platform-tests/streams/piping/general-addition.any.js")
      //       .default,
      //     done
      //   );
      // });

      it("should pass general.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/piping/general.any.js").default,
          done
        );
      });

      it("should pass multiple-propagation.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/piping/multiple-propagation.any.js")
            .default,
          done
        );
      });

      it("should pass pipe-through.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/piping/pipe-through.any.js")
            .default,
          done
        );
      });

      it("should pass then-interception.any.js tests", (done) => {
        runTest(
          require("./web-platform-tests/streams/piping/then-interception.any.js")
            .default,
          done
        );
      });

      // requires TransformStream
      // it("should pass throwing-options.any.js tests", (done) => {
      //   runTest(
      //     require("./web-platform-tests/streams/piping/throwing-options.any.js")
      //       .default,
      //     done
      //   );
      // });

      // requires TransformStream
      // it("should pass transform-streams.any.js tests", (done) => {
      //   runTest(
      //     require("./web-platform-tests/streams/piping/transform-streams.any.js")
      //       .default,
      //     done
      //   );
      // });
    });
  });
});
