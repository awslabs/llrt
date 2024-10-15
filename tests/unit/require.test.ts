globalThis._require = require; //used to preserve require during bundling/minification

const CWD = process.cwd();

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
  const a = _require(`${CWD}/fixtures/e.cjs`);

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
