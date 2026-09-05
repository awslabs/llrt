import { runSuite, runTestDynamic } from "./encoding.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "idlharness.any.js", // ReferenceError: idl_test is not defined
  ["encodeInto.any.js", [/MessageChannel is not defined/]],
  ["iso-2022-jp-decoder.any.js", [/encoding is not supported/]],
  ["replacement-encodings.any.js", [/XMLHttpRequest is not defined/]],
  [
    "single-byte-decoder.any.js",
    [/encoding is not supported/, /XMLHttpRequest is not defined/],
  ],
  ["textdecoder-eof.any.js", [/encoding is not supported/]],
  ["textdecoder-fatal-single-byte.any.js", [/encoding is not supported/]],
  ["textdecoder-labels.any.js", [/encoding is not supported/]],
  ["textdecoder-mistakes.any.js", [/encoding is not supported/]],
  ["textencoder-constructor-non-utf.any.js", [/encoding is not supported/]],
  ["unsupported-encodings.any.js", [/XMLHttpRequest is not defined/]],
]);
