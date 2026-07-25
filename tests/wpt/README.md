# web-platform-tests

llrt runs a subset of the [web-platform-tests](https://github.com/web-platform-tests/wpt)
to verify compatibility with web standards (WHATWG URL, Fetch, Streams, Encoding,
WebCrypto, …) against the upstream conformance suite.

## Layout

- `wpt/` — sparse-checkout of upstream WPT. Test files, support scripts, and data
  are consumed from here. **Do not copy `wpt/` files into `tests/wpt/`**;
  keep them in sync with upstream via `make update-wpt`.
- `tests/wpt/*.test.ts` — per-category entry points.
  Each is a thin one-liner that calls `runSuite(import.meta.url, harness,
skipFiles)` and drives the matching `wpt/...` suite through the harness.
- `tests/wpt/*.harness.js` — per-category harness definitions built with
  `makeRunner({ ... })`.
  These declare which WPT support scripts to load and any category-specific
  environment extras (`fetch`, `location`, etc.).
- `tests/wpt/_harness-wpt.js` — WPT-specific wrapper around the shared harness.
  It derives the `wpt/<category>/` path from the wrapper filename and exports
  `loadMetaScripts()` for harnesses that need META script support.
- `tests/wpt/third_party/test262/_harness-test262.js` — test262-specific wrapper.
  It maps wrapper files under `tests/wpt/third_party/test262/...` into
  `wpt/third_party/test262/test/...` and runs `.js` sources rather than
  `.any.js`.
- `tests/wpt/_harness-util.js` — shared harness machinery imported by the
  wrappers, not directly by `.test.ts` entry points.
  It provides:
  - `loadWptScript(rel, ctx)` — read and compile a WPT support script into a
    context, cached per process.
  - `wrapTestSuite(src)` — compile a WPT test body with `with (context)` so
    bare testharness APIs resolve; honours `'use strict'`.
  - `createContext({extras, scripts})` — build a fresh context with standard
    WPT helpers plus category-specific extras.
  - `makeRunner({context, postSetup, wrap})` — produce `runTestDynamic(source,
done, ctx)`.
  - `runSuite(metaUrl, harness, skipFiles?)` — `describe`/`it` driver for a
    WPT directory.

## Running WPTs

```sh
make test-wpt
```

This starts the WPT server in the background and runs the suite. `setup-wpt`
runs first if needed, initialising the `wpt/` submodule and configuring
sparse-checkout. The full run is printed to the console and distilled into the
sorted failing-test list `wpt_errors.txt`.

Other targets:

| target       | what it does                                                  |
| ------------ | ------------------------------------------------------------- |
| `setup-wpt`  | init the submodule, set up sparse-checkout, `/etc/hosts`      |
| `update-wpt` | pull upstream master into the `wpt/` submodule                |
| `check-wpt`  | run the suite and diff results against the committed baseline |

## Regression baseline

`wpt_errors.txt` is committed as the expected-failures baseline: the sorted,
de-duplicated list of WPT test files that currently fail (just the
`suite > should pass <file>.any.js tests` identifiers — no error messages,
stack traces, or version strings, so it stays stable across machines and CI).
The runner reports one result per `.any.js` file (it stops a file at its first
failing sub-test), so file identity is the reliable signal; the raw messages
vary by environment (`"Failed to fetch"` vs `"client error (Connect)"`, absolute
stack-trace paths, etc.) and would cause false diffs.

`make check-wpt` runs the suite, regenerates the report, and diffs it against
the committed `wpt_errors.txt`. It fails on **any** difference:

- a **new** failure is a regression — fix the code; or
- a **disappeared** failure means a test now passes — the baseline is stale.

CI runs `make check-wpt`, so the build goes red whenever results drift from the
baseline. To accept an intended change (a fix that makes tests pass, or a newly
accepted failure), regenerate and commit the baseline:

```sh
make check-wpt   # regenerates wpt_errors.txt (left in place on mismatch)
git add wpt_errors.txt
```

The per-suite `skipFiles` lists (in each `*.test.ts`) still exist for whole
files that should not run at all (e.g. tentative specs); the baseline covers
everything that runs but fails.

Running a single `.test.js` directly (skipping `make`):

```sh
JS_MINIFY=0 TEST_SUB_DIR=wpt node build.mjs
./target/debug/llrt test -d bundle/js/__tests__/wpt <filter>
```

## Adding a new WPT category

Say you want to pull in the `performance-timeline` WPT tests.

### 1. Sparse-checkout the category

In the repo `Makefile`, add `/performance-timeline` to the `setup-wpt` target's
`git sparse-checkout set` list (keep the block alphabetised). Then:

```sh
make setup-wpt    # or: cd wpt && git sparse-checkout reapply
```

### 2. Add the harness

Create `tests/wpt/performance-timeline.harness.js`:

```js
import { makeRunner } from "./_harness-util.js";

export const runTestDynamic = makeRunner({
  context: () => ({
    // WPT support scripts loaded into the context on top of the defaults
    // (testharness.js, idlharness.js, gc.js, subset-tests.js).
    scripts: [
      // Any per-category helpers the tests rely on, resolved inside `wpt/`.
      // e.g. "performance-timeline/resources/performance-observer.js",
    ],
  }),
});
```

If tests need custom globals (e.g. a `fetch` shim, a `location` object,
category-specific overrides of `self.X` after setup), use `extras` and
`postSetup` — see `fetch.harness.js` / `encoding.harness.js` for examples.

### 3. Add the test entry

Create `tests/wpt/performance-timeline.test.ts`:

```ts
import { runSuite } from "./_harness-util.js";
import { runTestDynamic } from "./performance-timeline.harness.js";

runSuite(import.meta.url, runTestDynamic, [
  // "known-to-fail.any.js", // reason
  // /\.tentative\./,        // whole pattern
]);
```

The filename determines the directory walked under `wpt/`: dots become path
separators. `performance-timeline.test.ts` → `wpt/performance-timeline/`,
`fetch.api.basic.test.ts` → `wpt/fetch/api/basic/`.

For `third_party/test262`, wrappers under `tests/wpt/third_party/test262/...` are
mapped into `wpt/third_party/test262/test/...` by the `tests/wpt/third_party/
test262/_harness-test262.js` helper.

### 4. Run the new suite

```sh
make test-wpt
```

New failures show up as additions in the `make check-wpt` diff against the
committed `wpt_errors.txt` baseline. Decide whether to fix the code, add the
whole file to `skipFiles` (with a one-line reason), or accept the failure by
committing the updated baseline.

## Updating WPT

```sh
make update-wpt
make test-wpt
```

`update-wpt` fetches the latest `master` for the submodule. New tests in the
already-checked-out categories appear automatically on the next run; pre-existing
tests that have changed may start passing or failing differently. Run
`make check-wpt` to see what moved relative to the committed baseline, then
commit the updated `wpt_errors.txt` if the changes are expected.

## Debugging

- `DEBUG=1 make test-wpt` — enables `testharness.js` debug mode.
- `./target/debug/llrt test -d bundle/js/__tests__/wpt <substring>` — runs only
  tests whose filename contains `<substring>`.
- `sed 's/\x1b\[[0-9;]*m//g' wpt_server.log` — strip ANSI from server logs.
