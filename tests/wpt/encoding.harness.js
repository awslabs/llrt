import { makeRunner } from "./_harness-util.js";

// Encodings llrt actually supports. The raw WPT encodings.js lists every
// encoding in the spec; we filter to the supported subset so tests don't
// blow up on encodings we intentionally don't implement.
const SUPPORTED = new Set([
  "UTF-8",
  "windows-1252",
  "replacement",
  "UTF-16BE",
  "UTF-16LE",
]);

export const runTestDynamic = makeRunner({
  context: () => ({
    scripts: [
      "encoding/resources/encodings.js",
      "encoding/resources/decoding-helpers.js",
    ],
  }),
  postSetup(context) {
    if (Array.isArray(context.encodings_table)) {
      context.encodings_table = context.encodings_table
        .map((s) => ({
          ...s,
          encodings: s.encodings.filter((e) => SUPPORTED.has(e.name)),
        }))
        .filter((s) => s.encodings.length > 0);
    }
  },
});
