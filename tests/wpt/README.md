# web-platform-tests

llrt runs a subset of the [web-platform-tests](https://github.com/web-platform-tests/wpt)
to verify compatibility with web standards (WHATWG URL, Fetch, Streams, Encoding,
WebCrypto, …) against the upstream conformance suite.

## Layout

- `wpt/` (repo root) — sparse-checkout of upstream WPT. All test files, support
  scripts (`resources/testharness.js`, `common/gc.js`, …), and test data
  (`url/resources/urltestdata.json`, …) are consumed from here. **Do not copy
  these files into `tests/wpt/`** — they must stay in sync with upstream via
  `make update-wpt`.
- `tests/wpt/*.test.ts` — per-category entry points. Each is a one-liner that
  calls `runSuite(import.meta.url, harness, skipFiles)` which walks
  `wpt/<category>/` for `.any.js` files and drives each through the harness.
- `tests/wpt/*.harness.js` — per-category harness built with `makeRunner({…})`.
  Declares which WPT support scripts to load, plus any extras (a `fetch` shim,
  a `location`, etc.) the tests in that category need.
- `tests/wpt/_harness-util.js` — the shared machinery:
  - `loadWptScript(rel, ctx)` — read + compile + run a WPT script into a `ctx`
    (scripts are cached so each is compiled once per process).
  - `wrapTestSuite(src)` — compile a WPT test body with `with (context)` so
    bare `assert_true`/`promise_test`/… resolve; honours `'use strict'`.
  - `createContext({extras, scripts})` — fresh context preloaded with
    `resources/idlharness.js`, `resources/testharness.js`, `common/gc.js`,
    `common/subset-tests.js`, plus the category's extra scripts.
  - `makeRunner({context, postSetup, wrap})` — build a harness's
    `runTestDynamic(source, done, ctx)`.
  - `runSuite(metaUrl, harness, skipFiles?)` — `describe/it` driver. `skipFiles`
    accepts strings or `RegExp`s.

## Running WPTs

```sh
make test-wpt
```

This starts the WPT server in the background and runs the suite. `setup-wpt`
runs first if needed, initialising the `wpt/` submodule and configuring
sparse-checkout. Output is post-processed into `wpt_errors.txt`.

Other targets:

| target       | what it does                                             |
| ------------ | -------------------------------------------------------- |
| `setup-wpt`  | init the submodule, set up sparse-checkout, `/etc/hosts` |
| `update-wpt` | pull upstream master into the `wpt/` submodule           |
| `tidyup-wpt` | strip ANSI + normalise paths in the error report         |

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

### 4. Run the new suite

```sh
make test-wpt
```

New failures show up in `wpt_errors.txt`. Add them to the `skipFiles` list
with a one-line reason as you decide to fix, defer, or skip them.

## Updating WPT

```sh
make update-wpt
make test-wpt
```

`update-wpt` fetches the latest `master` for the submodule. New tests in the
already-checked-out categories appear automatically on the next run; pre-existing
tests that have changed may start passing or failing differently. Compare the
new `wpt_errors.txt` against the last committed one to see what moved.

## Debugging

- `DEBUG=1 make test-wpt` — enables `testharness.js` debug mode.
- `./target/debug/llrt test -d bundle/js/__tests__/wpt <substring>` — runs only
  tests whose filename contains `<substring>`.
- `sed 's/\x1b\[[0-9;]*m//g' wpt_server.log` — strip ANSI from server logs.
