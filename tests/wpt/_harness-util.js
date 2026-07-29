// Shared helpers for running WPT tests against llrt's test runner.
// Support scripts (testharness.js, idlharness.js, gc.js, …) are loaded
// directly from the `wpt/` submodule so they stay in sync with upstream.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const CWD = process.cwd();
const WPT_DIR = path.join(CWD, "wpt");
const TEST_DIR = path.join(CWD, "bundle", "js", "__tests__", "wpt");

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

function attachCompletion(context, done, ctx = {}, { allowNoTests = false } = {}) {
  context.add_completion_callback((tests) => {
    const real = tests.filter(
      ({ name, status }) => !(name === "Loading data..." && status === 0)
    );
    if (!allowNoTests && real.length === 0)
      return done(new Error("No tests were executed!"));
    const failure = real.find((t) => {
      if (t.status === 0) return false;
      if (ctx.skipErrors && ctx.skipErrors.length > 0) {
        const errStr = `[${t.name}] ${t.message || String(t)}`;
        return !ctx.skipErrors.some((skip) =>
          typeof skip === "string"
            ? errStr.includes(skip) || t.name.includes(skip)
            : skip.test(errStr)
        );
      }
      return true;
    });
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
      } catch { }
    }
  }
  return out;
}

// Build a `runTestDynamic(source, done, ctx?)` from a declarative harness
// config. `config.context(ctx)` returns the per-test context options,
// `config.postSetup(context, ctx)` runs after creation, `config.wrap(source,
// ctx)` can transform source and return `[source, extraScripts]`.
function baseRunner(config, options = {}) {
  return (source, done, ctx = {}) => {
    const context = createContext(config.context ? config.context(ctx) : {});
    config.postSetup?.(context, ctx);
    attachCompletion(context, done, ctx, options);
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
function baseRunSuite(
  metaUrl,
  harness,
  skipFiles = [],
  { filePattern, subDir } = {}
) {
  const finalSubDir = subDir ?? deriveDefaultSubDir(metaUrl);
  const finalPattern = filePattern ?? /\.any\.js$/;
  const targetDir = path.join(WPT_DIR, finalSubDir);

  const matchesFile = (file, pattern) => {
    if (typeof pattern === "string") return pattern === file;
    if (pattern instanceof RegExp) return pattern.test(file);
    return false;
  };

  const skip = (f) =>
    /\.tentative\./.test(f) ||
    skipFiles.some((s) => !Array.isArray(s) && matchesFile(f, s));
  const testFiles = fs
    .readdirSync(targetDir)
    .filter((f) => finalPattern.test(f) && !skip(f));

  describe(finalSubDir, () => {
    for (const file of testFiles) {
      it(`should pass ${file} tests`, (done) => {
        const source = fs.readFileSync(path.join(targetDir, file), "utf8");
        const fileErrorSkipRules = skipFiles.filter(
          (s) => Array.isArray(s) && matchesFile(file, s[0])
        );
        const skipErrors = fileErrorSkipRules.flatMap((s) =>
          Array.isArray(s[1]) ? s[1] : [s[1]]
        );
        harness(source, done, {
          baseDir: WPT_DIR,
          testDir: targetDir,
          ...(skipErrors.length > 0 ? { skipErrors } : {}),
        });
      });
    }
  });
}

function deriveDefaultSubDir(metaUrl) {
  const basename = path.basename(fileURLToPath(metaUrl));
  return basename
    .replace(/\.test\.[jt]s$/, "")
    .split(".")
    .join(path.sep);
}

export function makeRunnerWPT(config) {
  return baseRunner(config);
}

export function runSuiteWPT(metaUrl, harness, skipFiles = []) {
  return baseRunSuite(metaUrl, harness, skipFiles, {
    filePattern: /\.any\.js$/,
    subDir: deriveSubDir(metaUrl),
  });
}

export function deriveSubDir(metaUrl) {
  const metaPath = fileURLToPath(metaUrl);
  const relativePath = path.relative(TEST_DIR, metaPath);

  const match = relativePath.match(/^(.+)\.test\.[jt]s$/);

  return path.join(...match[1].split("."));
}

export function makeRunnerTest262(config) {
  return baseRunner(config, { allowNoTests: true });
}

export function runSuiteTest262(metaUrl, harness, skipFiles = []) {
  return baseRunSuite(metaUrl, harness, skipFiles, {
    filePattern: /\.js$/,
    subDir: deriveTest262SubDir(metaUrl),
  });
}

function deriveTest262SubDir(metaUrl) {
  const metaPath = fileURLToPath(metaUrl);
  const relativePath = path.relative(TEST_DIR, metaPath);

  const match = relativePath.match(/^test262[\\/](.+)\.test\.[jt]s$/);

  return path.join("third_party", "test262", "test", ...match[1].split("."));
}
