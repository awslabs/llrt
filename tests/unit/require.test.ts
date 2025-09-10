globalThis._require = require; //used to preserve require during bundling/minification
const CWD = process.cwd();
import { spawn } from "node:child_process";

import { platform } from "os";
const IS_WINDOWS = platform() === "win32";

it("should require a file (absolute path)", () => {
  const { hello } = _require(`${CWD}/fixtures/hello.js`);

  expect(hello).toEqual("hello world!");
});

it("should require a json file (absolute path)", () => {
  const a = _require(`${CWD}/package.json`);

  expect(a.private).toEqual(true);
});

it("should require a js file (relative path)", () => {
  const { hello } = _require("../../../../fixtures/hello.js");

  expect(hello).toEqual("hello world!");
});

it("should require a json file (relative path)", () => {
  const a = _require("../../../../fixtures/package.json");

  expect(a.private).toEqual(true);
});

it("should require a json file (path unspecified)", () => {
  const a = _require("package.json");

  expect(a.private).toEqual(true);
});

it("should require a file (file schema)", () => {
  const { hello } = _require(`file://${CWD}/fixtures/hello.js`);

  expect(hello).toEqual("hello world!");
});

it("should require a json file (file schema)", () => {
  const a = _require(`file://${CWD}/package.json`);

  expect(a.private).toEqual(true);
});

it("should return same module when require multiple files", () => {
  const { hello: hello1 } = _require(`${CWD}/fixtures/hello.js`);
  const { hello: hello2 } = _require(`${CWD}/fixtures/hello.js`);
  const { hello: hello3 } = _require(`${CWD}/fixtures/hello.js`);

  expect(hello1).toEqual(hello2);
  expect(hello1).toEqual(hello3);
});

it("should handle cyclic requires", () => {
  const a = _require(`${CWD}/fixtures/a.js`);
  const b = _require(`${CWD}/fixtures/b.js`);

  expect(a.done).toEqual(b.done);
});

it("should handle cjs requires", () => {
  const a = _require(`${CWD}/fixtures/import.cjs`);

  expect(a.c).toEqual("c");
});

it("should handle cjs requires", () => {
  const a = _require(`${CWD}/fixtures/prop-export.cjs`);

  expect(a.prop).toEqual("a");
});

it("should be able to use node module with prefix `node:` with require", () => {
  let { Console } = require("node:console");
  const consoleObj = new Console({
    stdout: process.stdout,
    stderr: process.stderr,
  });

  // we check if the log does not throw an exception when called
  consoleObj.log("log");
  consoleObj.debug("debug");
  consoleObj.info("info");
  consoleObj.assert(false, "text for assertion should display");
  consoleObj.assert(true, "This text should not be seen");

  consoleObj.warn("warn");
  consoleObj.error("error");
  consoleObj.trace("trace");
});

it("should be able to import exported functions", () => {
  const importedFunction = _require(`${CWD}/fixtures/export-function.cjs`);
  expect(importedFunction()).toBe("hello world!");
});

it("should return same value for multiple require statements", () => {
  const filename = `${CWD}/fixtures/prop-export.cjs`;
  const a = _require(filename);
  const b = _require(filename);
  expect(a).toStrictEqual(b);
});

it("should return all props", () => {
  const a = _require(`${CWD}/fixtures/define-property-export.cjs`);
  expect(a.__esModule).toBe(true);
});

it("should import cjs modules using import statement", async () => {
  const filename = `${CWD}/fixtures/prop-export.cjs`;
  const a = await import(filename);
  const b = await import(filename);
  const c = _require(filename);
  expect(a).toStrictEqual(b);
  expect(a.default).toStrictEqual(c);
  expect(b.default).toStrictEqual(c);
});

it("should handle inner referenced exports", () => {
  const a = _require(`${CWD}/fixtures/referenced-exports.cjs`);
  expect(a.cat()).toBe("str");
  expect(a.length()).toBe(1);
});

if (!IS_WINDOWS) {
  it("should handle named exports from CJS imports", (cb) => {
    spawn(process.argv0, [
      "-e",
      `import {cat} from "${CWD}/fixtures/referenced-exports.cjs"`,
    ]).on("close", (code) => {
      expect(code).toBe(0);
      cb();
    });
  });
}

it("require builtin modules", () => {
  _require("path");
});

it("require `debug` module element", () => {
  _require(`${CWD}/fixtures/test_modules/test-debug.js`);
});

it("require `lodash.merge` module element", () => {
  _require(`${CWD}/fixtures/test_modules/test-lodash.merge.js`);
});

it("require `uuid` module element", () => {
  _require(`${CWD}/fixtures/test_modules/test-uuid.js`);
});

it("require `react-dom` module element", () => {
  _require(`${CWD}/fixtures/test_modules/test-react-dom.js`);
});

it("require `@aws-lambda-powertools` module element", () => {
  _require(
    `${CWD}/fixtures/test_modules/test-aws-lambda-powertools-jmespath.js`
  );
});

it("require `hono/utils/url` module element", () => {
  _require(`${CWD}/fixtures/test_modules/test-elem-hono.js`);
});

it("regression testing for issue #903", () => {
  expect(() => _require(`${CWD}/fixtures/test903/foo.mjs`)).toThrow(
    /Error resolving module /
  );
});

//create a test that spawns a subprocess and executes require.mjs from fixtures and captures stdout
it("should handle blocking requires", (done) => {
  const proc = spawn(process.argv0, [`${CWD}/fixtures/require.mjs`]);
  let stdout = "";
  proc.stdout.on("data", (data) => {
    stdout += data.toString();
  });
  proc.on("close", (code) => {
    try {
      expect(code).toBe(0);
      expect(stdout).toBe(
        ["1", "2", "3", "4", "5", "hello world!", "6", ""].join("\n")
      );
      done();
    } catch (e) {
      done(e);
    }
  });
});
