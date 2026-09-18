import { runSuite, runTestDynamic } from "./fetch.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "request-forbidden-headers.any.js", // ReferenceError: promise_test is not defined
  "request-upload.h2.any.js",
  "scheme-blob.sub.any.js", // TypeError: not a function
  "error-after-response.any.js", // hangs: response reader doesn't abort on network error
  "keepalive.any.js", // needs document global (browser-only)
  [
    "request-referrer.any.js",
    [
      /about:client referrer/, // Server runtime does not send a document referrer
      /url referrer/, // Server runtime does not send a document referrer
    ],
  ],
  [
    "conditional-get.any.js",
    [
      /Testing conditional GET with ETags/, // HTTP cache revalidation is not implemented
    ],
  ],
  [
    "header-value-combining.any.js",
    [
      /response\.headers\.get\('content-length'\) expects 0/, // HTTP parser rejects this Content-Length response
      /response\.headers\.get\('double-trouble'\) expects , /, // HTTP parser rejects this malformed header response
      /response\.headers\.get\('foo-test'\) expects 1, 2, 3/, // HTTP parser rejects this malformed header response
      /response\.headers\.get\('heya'\) expects , /, // HTTP parser rejects this malformed header response
      /response\.headers\.get\('www-authenticate'\) expects 1, 2, 3, 4/, // HTTP parser rejects this malformed header response
    ],
  ],
  [
    "integrity.sub.any.js",
    [
      /CORS /, // WPT CORS cases require the unavailable port 8001
      /integrity for opaque response/, // WPT HTTPS opaque cases require the unavailable TLS test port
    ],
  ],
  [
    "mode-no-cors.sub.any.js",
    [
      /Fetch https:\/\/web-platform\.test:8443/, // WPT HTTPS opaque case requires the unavailable TLS test port
      /Fetch http:\/\/web-platform\.test:8001/, // WPT cross-origin case requires the unavailable HTTP test port
    ],
  ],
  [
    "mode-same-origin.any.js",
    [
      /Fetch http:\/\/www1\.web-platform\.test:8000/, // Server runtime has no browser page origin for this cross-origin check
      /Fetch .*redirect\.py\?location=\.\.\/resources\/top\.txt/, // WPT relative redirect URL is not accepted by the runtime parser
      /redirect\.py\?location=http:\/\/www1\.web-platform\.test:8000/, // Server runtime has no browser page origin for this cross-origin redirect
    ],
  ],
  [
    "referrer.any.js",
    [
      /origin-when-cross-origin policy/, // Server runtime has no browser navigation referrer
      /Referrer with credentials/, // Server runtime does not derive a document referrer
      /Referrer with fragment ID/, // Server runtime does not derive a document referrer
    ],
  ],
  [
    "request-headers-case.any.js",
    [
      /Multiple headers with the same name, different case/, // HTTP library normalizes wire header names
    ],
  ],
]);
