// Shared helpers for running WPT tests against llrt's test runner.
// Support scripts (testharness.js, idlharness.js, gc.js, …) are loaded
// directly from the `wpt/` submodule so they stay in sync with upstream.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const CWD = process.cwd();
const WPT_DIR = path.join(CWD, "wpt");

// Compile a WPT support script into a reusable `(self) => void` function.
// `with (self)` lets `self.Foo = …; Foo` work (see idlharness.js); the
// appended assignments re-export top-level `const`/`let`/`var`/`function`
// declarations onto `self` because those don't propagate through `with`.
function compileWptScript(relativePath) {
  const src = fs.readFileSync(path.join(WPT_DIR, relativePath), "utf8");
  const names = new Set();
  for (const m of src.matchAll(
    /^\s*(?:const|let|var|function)\s+([A-Za-z_$][\w$]*)/gm
  )) {
    names.add(m[1]);
  }
  const exports = [...names]
    .map((n) => `try{self[${JSON.stringify(n)}]=${n}}catch(_){}`)
    .join(";");
  return new Function("self", `with(self){${src}\n;${exports}\n}`);
}

// Compilation is the expensive part (testharness.js is ~3000 lines), and
// support scripts never change between tests, so cache per-process.
const scriptCache = new Map();
export function loadWptScript(relativePath, context) {
  let fn = scriptCache.get(relativePath);
  if (!fn) {
    fn = compileWptScript(relativePath);
    scriptCache.set(relativePath, fn);
  }
  fn(context);
}

// Wrap a WPT test source into a runnable function. `with(context)` is needed
// because test bodies use bare identifiers (assert_true, test, fetch, …).
// Honour `'use strict'` so strict-mode semantics apply inside the test.
export function wrapTestSuite(sourceCode, extraScripts = "") {
  const strict = /^\s*(?:\/\/[^\n]*\n\s*)*["']use strict["']/.test(sourceCode);
  return new Function(
    "context",
    `with (context) {
      ${strict ? `(function(){"use strict";` : ""}
      ${extraScripts}
      ${sourceCode}
      ${strict ? "})();" : ""}
    }`
  );
}

// Build a WPT `context` preloaded with the standard harness + category
// scripts. Callers pass `extras` for per-category globals (fetch, location…).
export function createContext({ extras = {}, scripts = [] } = {}) {
  const context = {
    createBuffer: (type, length) => new self[type](length),
    setTimeout,
    DOMException,
    QuotaExceededError,
    location: {},
    ...extras,
  };
  loadWptScript("resources/idlharness.js", context);
  loadWptScript("resources/testharness.js", context);
  loadWptScript("common/gc.js", context);
  loadWptScript("common/subset-tests.js", context);
  for (const s of scripts) loadWptScript(s, context);
  context.setup({
    explicit_done: true,
    debug: process.env.DEBUG !== undefined,
  });
  globalThis.gc = globalThis.__gc;
  return context;
}

function attachCompletion(context, done) {
  context.add_completion_callback((tests) => {
    const real = tests.filter(
      ({ name, status }) => !(name === "Loading data..." && status === 0)
    );
    if (real.length === 0) return done(new Error("No tests were executed!"));
    const failure = real.find((t) => t.status !== 0);
    done(failure && `[${failure.name}] ${failure.message || String(failure)}`);
  });
}

// Resolve a `// META: script=...` path against `testDir` first (for same-dir
// helpers), then the `wpt/` root (for paths starting with `/`).
export function loadMetaScripts(source, testDir) {
  let out = "";
  if (!testDir) return out;
  for (const [, rel] of source.matchAll(/\/\/\s*META:\s*script=(.+)/g)) {
    const trimmed = rel.trim();
    for (const base of [testDir, WPT_DIR]) {
      try {
        out += fs.readFileSync(path.resolve(base, trimmed), "utf8") + "\n";
        break;
      } catch {}
    }
  }
  return out;
}

// Build a `runTestDynamic(source, done, ctx?)` from a declarative harness
// config. `config.context(ctx)` returns the per-test context options,
// `config.postSetup(context, ctx)` runs after creation, `config.wrap(source,
// ctx)` can transform source and return `[source, extraScripts]`.
export function makeRunner(config) {
  return (source, done, ctx = {}) => {
    const context = createContext(config.context ? config.context(ctx) : {});
    config.postSetup?.(context, ctx);
    attachCompletion(context, done);
    const [src, extras = ""] = config.wrap
      ? config.wrap(source, ctx)
      : [source, loadMetaScripts(source, ctx.testDir)];
    wrapTestSuite(src, extras)(context);
    context.done();
  };
}

// Drives a `describe(subDir) { it(file) { run(file) } }` block by walking
// `wpt/<subDir>/` for `.any.js` files. `metaUrl` is the test module's
// `import.meta.url` (so we can derive `subDir` from its filename).
export function runSuite(metaUrl, harness, skipFiles = []) {
  const basename = path.basename(fileURLToPath(metaUrl));
  const subDir = basename
    .replace(/\.test\.[jt]s$/, "")
    .split(".")
    .join(path.sep);
  const targetDir = path.join(WPT_DIR, subDir);
  const skip = (f) =>
    /\.tentative\./.test(f) ||
    skipFiles.some((s) => (s instanceof RegExp ? s.test(f) : s === f));
  const testFiles = fs
    .readdirSync(targetDir)
    .filter((f) => f.endsWith(".any.js") && !skip(f));

  describe(subDir, () => {
    for (const file of testFiles) {
      it(`should pass ${file} tests`, (done) => {
        const source = fs.readFileSync(path.join(targetDir, file), "utf8");
        harness(source, done, { baseDir: WPT_DIR, testDir: targetDir });
      });
    }
  });
}
