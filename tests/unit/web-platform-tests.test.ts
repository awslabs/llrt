describe("web-platform-tests", () => {
  beforeAll(() => {
    globalThis.location = {};

    // Set up the test harness
    require("./web-platform-tests/resources/idlharness.js");
    require("./web-platform-tests/resources/testharness.js");
    require("./web-platform-tests/common/subset-tests.js");

    // The test harness uses common/sab.js which uses WebAssembly which doesn't
    // work, so we can just create buffers the usual way
    globalThis.createBuffer = (type, length) => new self[type](length);

    globalThis.encodings_table =
      require("./web-platform-tests/encoding/resources/encodings.js").default;

    // Tests use fetch() to load JSON files, so we need to mock it to load files
    // from disk
    globalThis._fetch = globalThis.fetch;
    globalThis.fetch = (url) => {
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
    };
  });

  afterAll(() => {
    globalThis.fetch = globalThis._fetch;
  });

  const setupWPTTest = (done) => {
    setup({ explicit_done: true, debug: process.env.DEBUG !== undefined });
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });
  };

  // Not testing these edge cases
  // require("./web-platform-tests/url/historical.any.js");
  // Request.formData() not supported
  // require("./web-platform-tests/url/urlencoded-parser.any.js");

  /**
   * Encoding
   */

  it("should pass encoding/api-basics.any.js", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/encoding/api-basics.any.js");
    globalThis.done();
  });

  it("should pass encoding/api-invalid-label.any.js", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/encoding/api-invalid-label.any.js");
    globalThis.done();
  });

  it("should pass encoding/api-replacement-encodings.any.js", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/encoding/api-replacement-encodings.any.js");
    globalThis.done();
  });

  it("should pass encoding/api-surrogates-utf8.any.js", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/encoding/api-surrogates-utf8.any.js");
    globalThis.done();
  });

  it("should pass encoding/encodeInto.any.js", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/encoding/encodeInto.any.js");
    globalThis.done();
  });

  // Current support is utf8 and utf16le
  // it("should pass encoding/iso-2022-jp-decoder.any.js", (done) => {
  //   setupWPTTest(done);
  //   require("./web-platform-tests/encoding/iso-2022-jp-decoder.any.js");
  //   globalThis.done();
  // });

  // Requires XMLHTTPRequest which is not defined
  // it("should pass encoding/replacement-encodings.any.js", (done) => {
  //   setupWPTTest(done);
  //   require("./web-platform-tests/encoding/replacement-encodings.any.js");
  //   globalThis.done();
  // });

  it("should pass encoding/textdecoder-arguments.any.js", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/encoding/textdecoder-arguments.any.js");
    globalThis.done();
  });

  it("should pass encoding/textdecoder-byte-order-marks.any.js", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/encoding/textdecoder-byte-order-marks.any.js");
    globalThis.done();
  });

  // stream option not implemented
  // it("should pass encoding/textdecoder-copy.any.js", (done) => {
  //   setupWPTTest(done);
  //   require("./web-platform-tests/encoding/textdecoder-copy.any.js");
  //   globalThis.done();
  // });

  it("should pass encoding/textdecoder-eof.any.js", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/encoding/textdecoder-eof.any.js");
    globalThis.done();
  });

  // Current support is utf8 and utf16le
  // it("should pass encoding/textdecoder-fatal-single-byte.any.js", (done) => {
  //   setupWPTTest(done);
  //   require("./web-platform-tests/encoding/textdecoder-fatal-single-byte.any.js");
  //   globalThis.done();
  // });

  it("should pass encoding/textdecoder-fatal-streaming.any.js", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/encoding/textdecoder-fatal-streaming.any.js");
    globalThis.done();
  });

  it("should pass encoding/textdecoder-fatal.any.js", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/encoding/textdecoder-fatal.any.js");
    globalThis.done();
  });

  it("should pass encoding/textdecoder-ignorebom.any.js", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/encoding/textdecoder-ignorebom.any.js");
    globalThis.done();
  });

  // Not implemented
  // it("should pass encoding/textdecoder-labels.any.js", (done) => {
  //   setupWPTTest(done);
  //   require("./web-platform-tests/encoding/textdecoder-labels.any.js");
  //   globalThis.done();
  // });

  // stream option not implemented
  // it("should pass encoding/textdecoder-streaming.any.js", (done) => {
  //   setupWPTTest(done);
  //   require("./web-platform-tests/encoding/textdecoder-streaming.any.js");
  //   globalThis.done();
  // });

  it("should pass encoding/textdecoder-utf16-surrogates.any.js", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/encoding/textdecoder-utf16-surrogates.any.js");
    globalThis.done();
  });

  it("should pass encoding/textencoder-constructor-non-utf.any.js", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/encoding/textencoder-constructor-non-utf.any.js");
    globalThis.done();
  });

  // it("should pass encoding/textencoder-utf16-surrogates.any.js", (done) => {
  //   setupWPTTest(done);
  //   require("./web-platform-tests/encoding/textencoder-utf16-surrogates.any.js");
  //   globalThis.done();
  // });

  // Requires XMLHTTPRequest which is not defined
  // it("should pass encoding/unsupported-encodings.any.js", (done) => {
  //   setupWPTTest(done);
  //   require("./web-platform-tests/encoding/unsupported-encodings.any.js");
  //   globalThis.done();
  // });

  /**
   * URL
   */

  it("should pass url/url-constructor.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/url-constructor.any.js");
    globalThis.done();
  });

  it("should pass url/url-origin.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/url-origin.any.js");
    globalThis.done();
  });

  it("should pass url/url-searchparams.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/url-searchparams.any.js");
    globalThis.done();
  });

  it("should pass url/url-setters.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/url-setters.any.js");
    globalThis.done();
  });

  it("should pass url/url-setters-stripping.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/url-setters-stripping.any.js");
    globalThis.done();
  });

  it("should pass url/url-statics-canparse.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/url-statics-canparse.any.js");
    globalThis.done();
  });

  it("should pass url/url-statics-parse.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/url-statics-parse.any.js");
    globalThis.done();
  });

  it("should pass url/url-tojson.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/url-tojson.any.js");
    globalThis.done();
  });

  it("should pass url/urlsearchparams-append.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/urlsearchparams-append.any.js");
    globalThis.done();
  });

  it("should pass url/urlsearchparams-constructor.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/urlsearchparams-constructor.any.js");
    globalThis.done();
  });

  it("should pass url/urlsearchparams-delete.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/urlsearchparams-delete.any.js");
    globalThis.done();
  });

  it("should pass url/urlsearchparams-foreach.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/urlsearchparams-foreach.any.js");
    globalThis.done();
  });

  it("should pass url/urlsearchparams-get.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/urlsearchparams-get.any.js");
    globalThis.done();
  });

  it("should pass url/urlsearchparams-getall.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/urlsearchparams-getall.any.js");
    globalThis.done();
  });

  it("should pass url/urlsearchparams-has.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/urlsearchparams-has.any.js");
    globalThis.done();
  });

  it("should pass url/urlsearchparams-set.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/urlsearchparams-set.any.js");
    globalThis.done();
  });

  it("should pass url/urlsearchparams-size.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/urlsearchparams-size.any.js");
    globalThis.done();
  });

  it("should pass url/urlsearchparams-sort.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/urlsearchparams-sort.any.js");
    globalThis.done();
  });

  it("should pass url/urlsearchparams-stringifier.any.js tests", (done) => {
    setupWPTTest(done);
    require("./web-platform-tests/url/urlsearchparams-stringifier.any.js");
    globalThis.done();
  });
});
