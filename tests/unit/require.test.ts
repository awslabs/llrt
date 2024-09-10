globalThis._require = require;

const CWD = process.cwd();

it("should require a file", () => {
  const { hello } = _require(`${CWD}/fixtures/hello.js`);

  expect(hello).toEqual("hello world!");
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

it("should require json", () => {
  const a = _require(`${CWD}/package.json`);

  expect(a.private).toEqual(true);
});
