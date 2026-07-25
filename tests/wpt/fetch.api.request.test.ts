import { runSuite } from "./_harness-wpt.js";
import { runTestDynamic } from "./fetch.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "request-cache-default-conditional.any.js", // ReferenceError: promise_test is not defined
  "request-cache-default.any.js", // ReferenceError: promise_test is not defined
  "request-cache-force-cache.any.js", // ReferenceError: promise_test is not defined
  "request-cache-no-cache.any.js", // ReferenceError: promise_test is not defined
  "request-cache-no-store.any.js", // ReferenceError: promise_test is not defined
  "request-cache-only-if-cached.any.js", // ReferenceError: promise_test is not defined
  "request-cache-reload.any.js", // ReferenceError: promise_test is not defined
  [
    "request-headers.any.js",
    [
      '[Adding invalid request header "Cookie: KO"] assert_equals: expected (object) null but got (string) "KO"',
      '[Adding invalid request header "Cookie2: KO"] assert_equals: expected (object) null but got (string) "KO"',
      '[Check that request constructor is filtering headers provided as init parameter] assert_equals: expected (object) null but got (string) "potato"',
    ],
  ],
]);
