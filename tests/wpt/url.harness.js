import fs from "node:fs";
import path from "node:path";

import { makeRunner } from "./_harness-util.js";

const WPT_URL_RESOURCES = path.join(process.cwd(), "wpt", "url", "resources");

// WPT url tests fetch test data relative to `resources/`; point at the
// submodule so the payloads stay in sync with upstream.
function localFetch(url) {
  const m = /^resources\/([^?#]+)$/.exec(url);
  if (!m) throw new Error(`Cannot fetch URL: ${url}`);
  const data = fs.readFileSync(path.join(WPT_URL_RESOURCES, m[1]), "utf8");
  return Promise.resolve({
    json: () => Promise.resolve(JSON.parse(data)),
    text: () => Promise.resolve(data),
  });
}

export const runTestDynamic = makeRunner({
  context: () => ({
    extras: { fetch: localFetch },
    scripts: [
      "encoding/resources/encodings.js",
      "encoding/resources/decoding-helpers.js",
    ],
  }),
});
