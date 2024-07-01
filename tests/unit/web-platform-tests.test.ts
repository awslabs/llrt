describe("web-platform-tests", () => {
  beforeAll(() => {
    // Set up the test harness
    require("./web-platform-tests/resources/idlharness.js");
    require("./web-platform-tests/resources/testharness.js");
    setup({ debug: true });

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

  // Not testing these edge cases
  // require("./web-platform-tests/url/historical.any.js");
  // Request.formData() not supported
  // require("./web-platform-tests/url/urlencoded-parser.any.js");

  it("should pass url/url-constructor.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/url-constructor.any.js");
  });

  it("should pass url/url-origin.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/url-origin.any.js");
  });

  it("should pass url/url-searchparams.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/url-searchparams.any.js");
  });

  it("should pass url/url-setters.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/url-setters.any.js");
  });

  it("should pass url/url-setters-stripping.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/url-setters-stripping.any.js");
  });

  it("should pass url/url-statics-canparse.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/url-statics-canparse.any.js");
  });

  it("should pass url/url-statics-parse.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/url-statics-parse.any.js");
  });

  it("should pass url/url-tojson.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/url-tojson.any.js");
  });

  it("should pass url/urlsearchparams-append.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/urlsearchparams-append.any.js");
  });

  it("should pass url/urlsearchparams-constructor.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/urlsearchparams-constructor.any.js");
  });

  it("should pass url/urlsearchparams-delete.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/urlsearchparams-delete.any.js");
  });

  it("should pass url/urlsearchparams-foreach.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/urlsearchparams-foreach.any.js");
  });

  it("should pass url/urlsearchparams-get.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/urlsearchparams-get.any.js");
  });

  it("should pass url/urlsearchparams-getall.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/urlsearchparams-getall.any.js");
  });

  it("should pass url/urlsearchparams-has.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/urlsearchparams-has.any.js");
  });

  it("should pass url/urlsearchparams-set.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/urlsearchparams-set.any.js");
  });

  it("should pass url/urlsearchparams-size.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/urlsearchparams-size.any.js");
  });

  it("should pass url/urlsearchparams-sort.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/urlsearchparams-sort.any.js");
  });

  it("should pass url/urlsearchparams-stringifier.any.js tests", (done) => {
    add_completion_callback((tests, status, assertions) => {
      const failure = tests.find((test) => test.status !== 0);
      reset();
      done(failure);
    });

    require("./web-platform-tests/url/urlsearchparams-stringifier.any.js");
  });
});
