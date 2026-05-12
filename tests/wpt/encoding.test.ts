import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./encoding.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  "idlharness.any.js", // ReferenceError: idl_test is not defined
  "iso-2022-jp-decoder.any.js", // The "iso-2022-jp" encoding is not supported
  "replacement-encodings.any.js", // ReferenceError: promise_test is not defined
  "unsupported-encodings.any.js", // ReferenceError: promise_test is not defined
  "textdecoder-eof.any.js", // The "Big5" encoding is not supported
  "textdecoder-fatal-single-byte.any.js", // The "IBM866" encoding is not supported
  "textdecoder-labels.any.js", // IBM866 / legacy encodings
  "textdecoder-mistakes.any.js", // legacy single-byte encodings
  "textencoder-constructor-non-utf.any.js", // IBM866 / legacy encodings
  "encodeInto.any.js", // needs MessageChannel for the detached-buffer test
]);
